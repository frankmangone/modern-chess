import { beforeEach, describe, expect, it, vi } from "vitest";
import { clerkProvider } from "./clerk";
import { AuthError } from "./provider";

// Clerk's own library is replaced by a stand-in that records what it is asked and answers as
// scripted, so what is tested is our use of it: which calls, in which order, and what the person is told.

interface Script {
  signInCreate: () => unknown;
  signUpCreate: () => unknown;
  signInAttempt: () => unknown;
  signUpAttempt: () => unknown;
}

const calls: string[] = [];
let script: Script;
const activated: unknown[] = [];

function clerkError(code: string): Error & { errors: { code: string }[] } {
  return Object.assign(new Error(code), { errors: [{ code }] });
}

vi.mock("@clerk/clerk-js", () => ({
  Clerk: class {
    session: unknown = null;
    client = {
      signIn: {
        create: (params: unknown) => {
          calls.push(`signIn.create ${JSON.stringify(params)}`);
          return Promise.resolve(script.signInCreate());
        },
        prepareFirstFactor: (params: unknown) => {
          calls.push(`signIn.prepare ${JSON.stringify(params)}`);
          return Promise.resolve({});
        },
        attemptFirstFactor: (params: unknown) => {
          calls.push(`signIn.attempt ${JSON.stringify(params)}`);
          return Promise.resolve(script.signInAttempt());
        },
        authenticateWithRedirect: (params: unknown) => {
          calls.push(`google ${JSON.stringify(params)}`);
          return Promise.resolve();
        },
      },
      signUp: {
        create: (params: unknown) => {
          calls.push(`signUp.create ${JSON.stringify(params)}`);
          return Promise.resolve(script.signUpCreate());
        },
        prepareEmailAddressVerification: (params: unknown) => {
          calls.push(`signUp.prepare ${JSON.stringify(params)}`);
          return Promise.resolve({});
        },
        attemptEmailAddressVerification: (params: unknown) => {
          calls.push(`signUp.attempt ${JSON.stringify(params)}`);
          return Promise.resolve(script.signUpAttempt());
        },
      },
    };
    load = () => Promise.resolve();
    addListener = () => undefined;
    setActive = (params: unknown) => {
      activated.push(params);
      return Promise.resolve();
    };
    handleRedirectCallback = (_params: unknown, navigate: (to: string) => Promise<unknown>) => navigate("/games/go");
  },
}));

const existing = { supportedFirstFactors: [{ strategy: "email_code", emailAddressId: "idn_1" }] };
const complete = { status: "complete", createdSessionId: "sess_1" };

async function started() {
  const provider = clerkProvider("pk_test_x");
  await provider.start();
  return provider;
}

beforeEach(() => {
  calls.length = 0;
  activated.length = 0;
  script = {
    signInCreate: () => existing,
    signUpCreate: () => ({}),
    signInAttempt: () => complete,
    signUpAttempt: () => complete,
  };
});

describe("Clerk provider", () => {
  it("emails a code to someone who has an account, by signing them in", async () => {
    const provider = await started();

    await provider.sendCode("ada@example.com");

    expect(calls).toEqual([
      'signIn.create {"identifier":"ada@example.com"}',
      'signIn.prepare {"strategy":"email_code","emailAddressId":"idn_1"}',
    ]);
  });

  it("makes an account for an email Clerk does not know, and emails the code the same way", async () => {
    script.signInCreate = () => {
      throw clerkError("form_identifier_not_found");
    };
    const provider = await started();

    await provider.sendCode("new@example.com");

    expect(calls.slice(1)).toEqual([
      'signUp.create {"emailAddress":"new@example.com"}',
      'signUp.prepare {"strategy":"email_code"}',
    ]);
  });

  it("finishes the sign-in with the code and makes the session live", async () => {
    const provider = await started();
    await provider.sendCode("ada@example.com");

    await provider.verifyCode("123456");

    expect(calls.at(-1)).toBe('signIn.attempt {"strategy":"email_code","code":"123456"}');
    expect(activated).toEqual([{ session: "sess_1" }]);
  });

  it("finishes a sign-up with the code, too", async () => {
    script.signInCreate = () => {
      throw clerkError("form_identifier_not_found");
    };
    const provider = await started();
    await provider.sendCode("new@example.com");

    await provider.verifyCode("123456");

    expect(calls.at(-1)).toBe('signUp.attempt {"code":"123456"}');
    expect(activated).toEqual([{ session: "sess_1" }]);
  });

  it("does not start a session when the sign-up still wants something else", async () => {
    script.signInCreate = () => {
      throw clerkError("form_identifier_not_found");
    };
    script.signUpAttempt = () => ({ status: "missing_requirements", createdSessionId: null });
    const provider = await started();
    await provider.sendCode("new@example.com");

    await expect(provider.verifyCode("123456")).rejects.toMatchObject({ code: "unknown" });
    expect(activated).toEqual([]);
  });

  it("puts Clerk's errors into the screen's words", async () => {
    const provider = await started();
    for (const [clerk, ours] of [
      ["form_code_incorrect", "invalid_code"],
      ["verification_expired", "expired_code"],
      ["too_many_requests", "too_many_tries"],
      ["something_new", "unknown"],
    ] as const) {
      await provider.sendCode("ada@example.com");
      script.signInAttempt = () => {
        throw clerkError(clerk);
      };
      const failure = await provider.verifyCode("000000").catch((error: unknown) => error);
      expect(failure).toBeInstanceOf(AuthError);
      expect((failure as AuthError).code).toBe(ours);
    }
  });

  it("says the email is not an email, when Clerk does", async () => {
    script.signInCreate = () => {
      throw clerkError("form_param_format_invalid");
    };
    const provider = await started();

    await expect(provider.sendCode("nope")).rejects.toMatchObject({ code: "invalid_email" });
    expect(calls).toHaveLength(1);
  });

  it("refuses to check a code that was never sent", async () => {
    const provider = await started();

    await expect(provider.verifyCode("123456")).rejects.toMatchObject({ code: "unknown" });
  });

  it("goes to Google and comes back to our callback page, then the page it left from", async () => {
    const provider = await started();

    await provider.continueWithGoogle("/games/chess");

    expect(calls).toEqual([
      `google ${JSON.stringify({
        strategy: "oauth_google",
        redirectUrl: `${window.location.origin}/sso-callback`,
        redirectUrlComplete: `${window.location.origin}/games/chess`,
      })}`,
    ]);
  });

  it("answers where Clerk wanted to go after Google, as a path of ours", async () => {
    const provider = await started();

    expect(await provider.completeRedirect()).toBe("/games/go");
  });

  it("is unavailable before it has loaded", async () => {
    await expect(clerkProvider("pk_test_x").sendCode("ada@example.com")).rejects.toMatchObject({ code: "unavailable" });
  });
});
