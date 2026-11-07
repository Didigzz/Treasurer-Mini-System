import { describe, expect, test } from "bun:test";

import {
  computeBillSummary,
  computeBoarderPeriodSummaries,
  computeShareState,
  equalSplit,
  matchesBoarderFilter,
  matchesShareFilter,
  previewPayment,
  recomputePaymentChanges,
  validateCustomShares,
} from "../calculations";
import type { PaymentRecord, ShareWithState } from "../types";

function payment(id: string, amountMinor: number, paidAt = 1000): PaymentRecord {
  return {
    id,
    billShareId: "share-1",
    amountMinor,
    paidAt,
    note: null,
    changeMinor: 0,
    changeReturnedAt: null,
    changeReturnNote: null,
    voidedAt: null,
    voidReason: null,
    createdAt: paidAt,
  };
}

function share(
  id: string,
  name: string,
  amountDueMinor: number,
  payments: PaymentRecord[],
): ShareWithState {
  return {
    id,
    billId: "bill-1",
    boarderId: id,
    boarderNameSnapshot: name,
    amountDueMinor,
    createdAt: 0,
    state: computeShareState(amountDueMinor, payments),
  };
}

describe("equalSplit", () => {
  test("distributes centavo remainders in selection order", () => {
    const result = equalSplit(100000, [
      { boarderId: "c", name: "Cara", createdAt: 3 },
      { boarderId: "a", name: "Ana", createdAt: 1 },
      { boarderId: "b", name: "Ben", createdAt: 2 },
    ]);

    if ("error" in result) throw new Error(result.error);

    expect(result.shares).toEqual([
      { boarderId: "c", amountMinor: 33334 },
      { boarderId: "a", amountMinor: 33333 },
      { boarderId: "b", amountMinor: 33333 },
    ]);
    expect(result.shares.reduce((total, item) => total + item.amountMinor, 0)).toBe(100000);
  });

  test("supports one-centavo totals", () => {
    const result = equalSplit(1, [{ boarderId: "a", name: "Ana", createdAt: 1 }]);
    if ("error" in result) throw new Error(result.error);
    expect(result.shares[0]?.amountMinor).toBe(1);
  });

  test("rejects totals too small for the selected count", () => {
    const result = equalSplit(2, [
      { boarderId: "a", name: "Ana", createdAt: 1 },
      { boarderId: "b", name: "Ben", createdAt: 2 },
      { boarderId: "c", name: "Cara", createdAt: 3 },
    ]);
    expect("error" in result).toBe(true);
  });
});

describe("validateCustomShares", () => {
  test("accepts an exact total", () => {
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 5000 },
        { boarderId: "b", amountMinor: 5000 },
      ]).valid,
    ).toBe(true);
  });

  test("rejects short, over, and zero shares", () => {
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 4000 },
        { boarderId: "b", amountMinor: 5000 },
      ]).valid,
    ).toBe(false);
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 6000 },
        { boarderId: "b", amountMinor: 5000 },
      ]).valid,
    ).toBe(false);
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 10000 },
        { boarderId: "b", amountMinor: 0 },
      ]).valid,
    ).toBe(false);
  });
});

describe("payment and change allocation", () => {
  test("exact payment has no change", () => {
    const state = computeShareState(25000, [payment("p1", 25000)]);
    expect(state.status).toBe("paid");
    expect(state.changeOutstandingMinor).toBe(0);
  });

  test("overpayment creates change", () => {
    const state = computeShareState(25000, [payment("p1", 30000)]);
    expect(state.status).toBe("paid");
    expect(state.balanceMinor).toBe(0);
    expect(state.changeOutstandingMinor).toBe(5000);
  });

  test("partial payments accumulate", () => {
    const state = computeShareState(25000, [payment("p1", 10000), payment("p2", 10000, 2000)]);
    expect(state.status).toBe("partial");
    expect(state.paidMinor).toBe(20000);
    expect(state.balanceMinor).toBe(5000);
  });

  test("change is assigned to the payment crossing the due amount", () => {
    const payments = [payment("p1", 10000), payment("p2", 20000, 2000)];
    const changes = recomputePaymentChanges(25000, payments);
    expect(changes.find((item) => item.id === "p1")?.computedChangeMinor).toBe(0);
    expect(changes.find((item) => item.id === "p2")?.computedChangeMinor).toBe(5000);
  });

  test("voided payments are excluded and change is recomputed", () => {
    const withVoid = [
      { ...payment("p1", 10000), voidedAt: 3000, voidReason: "Mistake" },
      payment("p2", 20000, 2000),
    ];
    const state = computeShareState(25000, withVoid);
    expect(state.paidMinor).toBe(20000);
    expect(state.balanceMinor).toBe(5000);
    expect(state.changeOutstandingMinor).toBe(0);
  });

  test("preview distinguishes partial, exact, and change", () => {
    expect(previewPayment(25000, [], 10000).message).toBe("still owed");
    expect(previewPayment(25000, [], 25000).message).toBe("paid in full");
    expect(previewPayment(25000, [], 30000).message).toBe("change to return");
  });
});

describe("bill totals and filters", () => {
  const shares = [
    share("a", "Ana", 25000, [payment("p1", 30000)]),
    share("b", "Ben", 25000, [payment("p2", 10000, 2000)]),
    share("c", "Cara", 25000, []),
  ];
  const summary = computeBillSummary(shares, null);

  test("keeps expected equal to received plus outstanding", () => {
    expect(summary.expectedMinor).toBe(75000);
    expect(summary.appliedMinor).toBe(35000);
    expect(summary.outstandingMinor).toBe(40000);
    expect(summary.appliedMinor + summary.outstandingMinor).toBe(summary.expectedMinor);
  });

  test("counts paid shares with change separately", () => {
    expect(summary.paidShareCount).toBe(1);
    expect(summary.partialShareCount).toBe(1);
    expect(summary.unpaidShareCount).toBe(1);
    expect(summary.changeOutstandingMinor).toBe(5000);
    expect(summary.status).toBe("open");
  });

  test("classifies filters independently", () => {
    expect(matchesShareFilter(shares[0]!, "paid")).toBe(true);
    expect(matchesShareFilter(shares[0]!, "change")).toBe(true);
    expect(matchesShareFilter(shares[1]!, "unpaid")).toBe(true);
    expect(matchesShareFilter(shares[1]!, "paid")).toBe(false);
  });

  test("archived bills have archived status", () => {
    expect(computeBillSummary(shares, 123).status).toBe("archived");
  });

  test("settles when every share is covered even with change", () => {
    const settled = computeBillSummary(
      [
        share("a", "Ana", 25000, [payment("p1", 30000)]),
        share("b", "Ben", 25000, [payment("p2", 25000, 2000)]),
      ],
      null,
    );
    expect(settled.status).toBe("settled");
    expect(settled.changeOutstandingMinor).toBe(5000);
  });

  test("aggregates boarders across bills", () => {
    const summaries = computeBoarderPeriodSummaries([
      {
        boarderId: "a",
        boarderName: "Ana",
        boarderArchived: false,
        amountDueMinor: 25000,
        state: shares[0]!.state,
      },
      {
        boarderId: "a",
        boarderName: "Ana",
        boarderArchived: false,
        amountDueMinor: 10000,
        state: computeShareState(10000, [payment("p3", 5000, 3000)]),
      },
    ]);

    expect(summaries).toHaveLength(1);
    expect(summaries[0]?.expectedMinor).toBe(35000);
    expect(summaries[0]?.appliedMinor).toBe(30000);
    expect(summaries[0]?.outstandingMinor).toBe(5000);
    expect(matchesBoarderFilter(summaries[0]!, "unpaid")).toBe(true);
  });
});
