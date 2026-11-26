import test from "node:test";
import assert from "node:assert/strict";
import { num } from "../src/num.ts";

test("num writes up to three decimals and drops trailing zeros and a bare point", () => {
  assert.equal(num(1), "1");
  assert.equal(num(1.5), "1.5");
  assert.equal(num(0.13), "0.13");
  assert.equal(num(2.0004), "2");
  assert.equal(num(10), "10");
  assert.equal(num(0), "0");
});

test("num rounds an exact tie to the even digit, as Python does", () => {
  assert.equal(num(0.0625), "0.062");
  assert.equal(num(0.1875), "0.188");
  assert.equal(num(1.0625), "1.062");
  assert.equal(num(0.3125), "0.312");
});

test("num rounds what is not a tie to the nearest", () => {
  assert.equal(num(0.0626), "0.063");
  assert.equal(num(0.0624), "0.062");
  assert.equal(num(1.2346), "1.235");
});

test("num carries into the whole part", () => {
  assert.equal(num(0.9996), "1");
  assert.equal(num(9.99951), "10");
});

test("num keeps the sign of a negative, even one that rounds to zero", () => {
  assert.equal(num(-1.5), "-1.5");
  assert.equal(num(-0.0001), "-0");
  assert.equal(num(-0), "-0");
});

test("num refuses what has no digits", () => {
  assert.throws(() => num(Number.NaN), /cannot write/);
  assert.throws(() => num(Number.POSITIVE_INFINITY), /cannot write/);
});
