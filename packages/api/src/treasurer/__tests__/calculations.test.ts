import { describe, expect, test } from "bun:test";

import {
  computeBillSummary,
  computeBoarderPeriodSummaries,
  computeHistoryTotals,
  computeShareState,
  equalSplit,
  listHistoryPeriods,
  matchesBoarderFilter,
  matchesHistoryFilter,
  matchesShareFilter,
  previewPayment,
  recomputePaymentChanges,
  sortHistoryBills,
  splitWithOverrides,
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

  test("rounds to whole pesos when centavos are disabled", () => {
    const result = equalSplit(
      10_000,
      [
        { boarderId: "a", name: "Ana", createdAt: 1 },
        { boarderId: "b", name: "Ben", createdAt: 2 },
        { boarderId: "c", name: "Cara", createdAt: 3 },
      ],
      { allowCentavos: false },
    );

    if ("error" in result) throw new Error(result.error);
    expect(result.shares.map((share) => share.amountMinor)).toEqual([3400, 3300, 3300]);
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

  test("rejects short and over shares", () => {
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
  });

  test("accepts a zero share when the total still balances", () => {
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 10000 },
        { boarderId: "b", amountMinor: 0 },
      ]).valid,
    ).toBe(true);
  });

  test("rejects negative amounts", () => {
    expect(
      validateCustomShares(10000, [
        { boarderId: "a", amountMinor: 12000 },
        { boarderId: "b", amountMinor: -2000 },
      ]).valid,
    ).toBe(false);
  });
});

describe("splitWithOverrides", () => {
  const boarders = [
    { boarderId: "ana", name: "Ana", createdAt: 1 },
    { boarderId: "ben", name: "Ben", createdAt: 2 },
    { boarderId: "cara", name: "Cara", createdAt: 3 },
    { boarderId: "dan", name: "Dan", createdAt: 4 },
  ];

  test("splits the leftover among the unpinned boarders in selection order", () => {
    const result = splitWithOverrides(100_000, boarders, new Map([["ana", 30_000]]));
    if ("error" in result) throw new Error(result.error);

    expect(result.shares).toEqual([
      { boarderId: "ana", amountMinor: 30_000 },
      { boarderId: "ben", amountMinor: 23_334 },
      { boarderId: "cara", amountMinor: 23_333 },
      { boarderId: "dan", amountMinor: 23_333 },
    ]);
    expect(result.poolCount).toBe(3);
    expect(result.leftoverMinor).toBe(70_000);
    expect(result.rounded).toBe(false);
  });

  test("rounds to whole pesos when centavos are disabled", () => {
    const result = splitWithOverrides(100_000, boarders, new Map([["ana", 30_000]]), {
      allowCentavos: false,
    });
    if ("error" in result) throw new Error(result.error);

    expect(result.shares.map((share) => share.amountMinor)).toEqual([
      30_000, 23_400, 23_300, 23_300,
    ]);
    expect(result.rounded).toBe(true);
  });

  test("allows zero shares when custom amounts consume the total", () => {
    const result = splitWithOverrides(10_000, boarders.slice(0, 3), new Map([["ana", 10_000]]));
    if ("error" in result) throw new Error(result.error);

    expect(result.shares).toEqual([
      { boarderId: "ana", amountMinor: 10_000 },
      { boarderId: "ben", amountMinor: 0 },
      { boarderId: "cara", amountMinor: 0 },
    ]);
  });

  test("hands out the last centavos one at a time", () => {
    const result = splitWithOverrides(1_000, boarders.slice(0, 3), new Map([["ana", 999]]));
    if ("error" in result) throw new Error(result.error);

    expect(result.shares.map((share) => share.amountMinor)).toEqual([999, 1, 0]);
  });

  test("rejects a custom total above the bill", () => {
    const result = splitWithOverrides(10_000, boarders, new Map([["ana", 12_000]]));
    expect("error" in result).toBe(true);
    if ("error" in result) expect(result.overByMinor).toBe(2_000);
  });

  test("requires the pinned amounts to balance when nobody is left in the pool", () => {
    const all = new Map([
      ["ana", 2_500],
      ["ben", 2_500],
      ["cara", 2_500],
      ["dan", 2_500],
    ]);
    expect("error" in splitWithOverrides(10_000, boarders, all)).toBe(false);
    expect("error" in splitWithOverrides(12_000, boarders, all)).toBe(true);
  });

  test("keeps pinned rows in place and preserves pool order", () => {
    const result = splitWithOverrides(9_000, boarders, new Map([["ben", 3_000]]));
    if ("error" in result) throw new Error(result.error);

    expect(result.shares).toEqual([
      { boarderId: "ana", amountMinor: 2_000 },
      { boarderId: "ben", amountMinor: 3_000 },
      { boarderId: "cara", amountMinor: 2_000 },
      { boarderId: "dan", amountMinor: 2_000 },
    ]);
  });

  test("rejects a fractional pin when whole pesos are required", () => {
    const result = splitWithOverrides(10_000, boarders, new Map([["ana", 1_050]]), {
      allowCentavos: false,
    });
    expect("error" in result).toBe(true);
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

function historyBill(id: string, periodYear: number, periodMonth: number, createdAt: number) {
  return { bill: { id, billTypeId: "water", periodYear, periodMonth, createdAt } };
}

describe("bill history helpers", () => {
  const bills = [
    historyBill("a", 2026, 10, 1),
    historyBill("b", 2026, 11, 2),
    historyBill("c", 2025, 12, 3),
  ];

  test("matches every bill for the default filter", () => {
    expect(
      bills.every((item) =>
        matchesHistoryFilter(item.bill, { year: null, month: null, billTypeId: null }),
      ),
    ).toBe(true);
  });

  test("filters by month and type", () => {
    expect(
      bills
        .filter((item) =>
          matchesHistoryFilter(item.bill, { year: 2026, month: 11, billTypeId: null }),
        )
        .map((item) => item.bill.id),
    ).toEqual(["b"]);

    expect(
      matchesHistoryFilter(bills[1]!.bill, { year: 2026, month: 10, billTypeId: null }),
    ).toBe(false);
    expect(
      matchesHistoryFilter(bills[1]!.bill, { year: null, month: null, billTypeId: "water" }),
    ).toBe(true);
    expect(
      matchesHistoryFilter(bills[1]!.bill, { year: null, month: null, billTypeId: "rent" }),
    ).toBe(false);
  });

  test("sorts newest period first, then newest created, without mutating the input", () => {
    const sameMonth = [
      historyBill("old", 2026, 11, 1),
      historyBill("new", 2026, 11, 5),
    ];

    expect(
      sortHistoryBills([...bills, ...sameMonth]).map((item) => item.bill.id),
    ).toEqual(["new", "b", "old", "a", "c"]);
    expect(bills.map((item) => item.bill.id)).toEqual(["a", "b", "c"]);
  });

  test("lists periods that have bills, newest first, counting archived bills", () => {
    expect(
      listHistoryPeriods([...bills.map((item) => item.bill), historyBill("d", 2026, 11, 4).bill]),
    ).toEqual([
      { year: 2026, month: 11, billCount: 2 },
      { year: 2026, month: 10, billCount: 1 },
      { year: 2025, month: 12, billCount: 1 },
    ]);
  });

  test("sums totals and returns zeros for an empty list", () => {
    expect(
      computeHistoryTotals([
        { expectedMinor: 1000, appliedMinor: 600, outstandingMinor: 400, changeOutstandingMinor: 0 },
        { expectedMinor: 500, appliedMinor: 500, outstandingMinor: 0, changeOutstandingMinor: 50 },
      ]),
    ).toEqual({
      expectedMinor: 1500,
      appliedMinor: 1100,
      outstandingMinor: 400,
      changeOutstandingMinor: 50,
    });

    expect(computeHistoryTotals([])).toEqual({
      expectedMinor: 0,
      appliedMinor: 0,
      outstandingMinor: 0,
      changeOutstandingMinor: 0,
    });
  });
});
