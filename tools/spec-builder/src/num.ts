/** How a number is written in drawn art, the way Python's `f"{v:.3f}".rstrip("0").rstrip(".")`
 *  writes it: three decimals, then the trailing zeros and a bare point dropped. An exact tie
 *  (0.0625 at three decimals) goes to the even digit, where `toFixed` would go up, and a
 *  negative that rounds to zero keeps its sign ("-0"), as Python's does. */
export function num(value: number): string {
  if (!Number.isFinite(value)) throw new Error(`cannot write ${value} in a drawing`);
  const negative = value < 0 || Object.is(value, -0);
  const exact = Math.abs(value).toFixed(30);
  const [whole = "0", fraction = ""] = exact.split(".");
  const kept = fraction.slice(0, 3);
  const rest = fraction.slice(3);
  let digits = BigInt(whole + kept);
  const half = rest.startsWith("5") && /^50*$/.test(rest);
  const up = rest.charAt(0) > "5" || (rest.charAt(0) === "5" && !half) || (half && digits % 2n === 1n);
  if (up) digits += 1n;
  const text = digits.toString().padStart(4, "0");
  const point = text.length - 3;
  let out = `${text.slice(0, point)}.${text.slice(point)}`;
  out = out.replace(/0+$/, "").replace(/\.$/, "");
  return negative ? `-${out}` : out;
}
