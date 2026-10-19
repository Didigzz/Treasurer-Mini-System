import { describe, expect, test } from "bun:test";

import {
  MAX_AMOUNT_MINOR,
  formatMoney,
  minorToInputValue,
  parseMoneyToMinor,
} from "../money";

describe("parseMoneyToMinor", () => {
  test("parses whole and fractional amounts", () => {
    expect(parseMoneyToMinor("250")).toBe(25000);
    expect(parseMoneyToMinor("250.5")).toBe(25050);
    expect(parseMoneyToMinor("250.50")).toBe(25050);
    expect(parseMoneyToMinor("1,234.56")).toBe(123456);
    expect(parseMoneyToMinor(".5")).toBe(50);
    expect(parseMoneyToMinor("0")).toBe(0);
  });

  test("rejects invalid or unsafe values", () => {
    expect(parseMoneyToMinor("")).toBeNull();
    expect(parseMoneyToMinor("   ")).toBeNull();
    expect(parseMoneyToMinor("-1")).toBeNull();
    expect(parseMoneyToMinor("1.234")).toBeNull();
    expect(parseMoneyToMinor("1,2,3")).toBeNull();
    expect(parseMoneyToMinor("abc")).toBeNull();
    expect(parseMoneyToMinor(".")).toBeNull();
    expect(parseMoneyToMinor("1.2.3")).toBeNull();
  });

  test("rejects amounts above the maximum", () => {
    expect(parseMoneyToMinor("100000000.00")).toBeNull();
    expect(parseMoneyToMinor("99999999.99")).toBe(MAX_AMOUNT_MINOR);
  });
});

describe("minorToInputValue", () => {
  test("produces two decimal places", () => {
    expect(minorToInputValue(25000)).toBe("250.00");
    expect(minorToInputValue(1)).toBe("0.01");
    expect(minorToInputValue(MAX_AMOUNT_MINOR)).toBe("99999999.99");
  });
});

describe("formatMoney", () => {
  test("formats PHP", () => {
    expect(formatMoney(25000, "PHP")).toContain("250.00");
  });
});
