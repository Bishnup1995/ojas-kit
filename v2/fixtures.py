"""Deterministic fixtures for the v2 "read" block. No randomness outside fixed seeds.

  python3 fixtures.py r1 DIR            -> DIR/pricing.py (with the planted bug) + DIR/README.md
  python3 fixtures.py r1-ref DIR        -> DIR/pricing.py (correct reference, used only by the grader)
  python3 fixtures.py r2 DIR            -> DIR/ledger.py
  python3 fixtures.py r2-expected       -> prints the expected answer set for r2, one name per line
"""
import os, random, sys

# ---------------------------------------------------------------- r1: pricing.py
CATS = ["bolts", "nuts", "washers", "screws", "rivets", "anchors", "hinges", "brackets", "springs", "pins",
        "clips", "gaskets", "bearings", "bushings", "couplers", "valves", "fittings", "hoses", "clamps", "cables"]
REGIONS = ["north", "south", "east", "west", "central", "coastal", "alpine"]


def r1_source(buggy: bool) -> str:
    rng = random.Random(1501)
    out = []
    w = out.append
    w('"""Pricing engine for the Acme wholesale catalogue.\n\nEvery public quote goes through quote(). Per-SKU unit rates come from the generated\nrate_<category>_<region> functions; order-level rules (volume breaks, loyalty, shipping,\ntax) are applied in the order-level section further down.\n"""')
    w("from decimal import Decimal, ROUND_HALF_UP\n")
    w('CENT = Decimal("0.01")')
    w("TAX = {" + ", ".join(f'"{r}": Decimal("0.{rng.randint(5, 12):02d}")' for r in REGIONS) + "}")
    w("BASE = {" + ", ".join(f'"{c}": Decimal("{rng.randint(20, 900) / 100:.2f}")' for c in CATS) + "}")
    w("")
    w("")
    breaks_pool = [25, 50, 75, 150, 200, 250, 300, 400, 600, 750, 1200, 2000, 2500, 5000]

    def rate_fn(c, r):
        b1, b2, b3 = sorted(rng.sample(breaks_pool, 3))
        m0 = rng.randint(95, 125) / 100
        f1, f2, f3 = rng.randint(94, 99) / 100, rng.randint(88, 94) / 100, rng.randint(80, 88) / 100
        surcharge = rng.randint(0, 40) / 100
        lines = [
            f"def rate_{c}_{r}(qty: int, base: Decimal) -> Decimal:",
            f'    """Unit rate for {c} shipped from the {r} warehouse.',
            "",
            f"    Breaks at {b1}, {b2} and {b3} units are negotiated per warehouse and are",
            "    independent of the order-level volume discount.",
            '    """',
            "    if qty <= 0:",
            f'        raise ValueError("qty must be positive for {c}/{r}")',
            f'    rate = base * Decimal("{m0:.2f}")',
            f"    if qty >= {b3}:",
            f'        rate *= Decimal("{f3:.2f}")',
            f"    elif qty >= {b2}:",
            f'        rate *= Decimal("{f2:.2f}")',
            f"    elif qty >= {b1}:",
            f'        rate *= Decimal("{f1:.2f}")',
            f'    rate += Decimal("{surcharge:.2f}")  # handling surcharge',
            "    return rate.quantize(CENT, rounding=ROUND_HALF_UP)",
            "",
            "",
        ]
        return lines

    pairs = [(c, r) for c in CATS for r in REGIONS]  # 140 functions
    for c, r in pairs[:118]:
        out.extend(rate_fn(c, r))
    # ---- order-level section, deep inside the file
    vol = ">" if buggy else ">="
    out.extend([
        "# ---------------------------------------------------------------------------",
        "# Order-level rules",
        "# ---------------------------------------------------------------------------",
        "",
        "",
        "def unit_rate(sku: str, qty: int) -> Decimal:",
        '    """Look up the unit rate for a SKU of the form <category>-<region>."""',
        '    cat, region = sku.split("-", 1)',
        '    fn = globals().get(f"rate_{cat}_{region}")',
        "    if fn is None:",
        '        raise KeyError(f"unknown sku {sku!r}")',
        "    return fn(qty, BASE[cat])",
        "",
        "",
        "def _volume_factor(qty: int) -> Decimal:",
        '    """Order-level volume discount on the line subtotal."""',
        "    if qty >= 1000:",
        '        return Decimal("0.80")',
        "    if qty >= 500:",
        '        return Decimal("0.85")',
        f"    if qty {vol} 100:",
        '        return Decimal("0.90")',
        '    return Decimal("1")',
        "",
        "",
        "def _loyalty_factor(years: int) -> Decimal:",
        '    """Loyalty discount, applied after the volume discount."""',
        "    if years >= 10:",
        '        return Decimal("0.95")',
        "    if years >= 3:",
        '        return Decimal("0.98")',
        '    return Decimal("1")',
        "",
        "",
        "def line_total(sku: str, qty: int, years: int = 0) -> Decimal:",
        '    """Subtotal for one order line, after volume and loyalty discounts."""',
        "    gross = unit_rate(sku, qty) * qty",
        "    net = gross * _volume_factor(qty) * _loyalty_factor(years)",
        "    return net.quantize(CENT, rounding=ROUND_HALF_UP)",
        "",
        "",
        "def shipping(subtotal: Decimal, region: str) -> Decimal:",
        '    """Flat shipping per region; free from 500.00 upwards."""',
        '    if subtotal >= Decimal("500"):',
        '        return Decimal("0.00")',
        '    return Decimal("12.50") if region in ("alpine", "coastal") else Decimal("7.50")',
        "",
        "",
        "def quote(lines, region: str, years: int = 0) -> Decimal:",
        '    """Total for an order: lines is a list of (sku, qty). Tax applies to goods and shipping."""',
        '    subtotal = sum((line_total(sku, qty, years) for sku, qty in lines), Decimal("0"))',
        "    total = (subtotal + shipping(subtotal, region)) * (1 + TAX[region])",
        "    return total.quantize(CENT, rounding=ROUND_HALF_UP)",
        "",
        "",
    ])
    for c, r in pairs[118:]:
        out.extend(rate_fn(c, r))
    return "\n".join(out).rstrip() + "\n"


R1_README = """# Acme pricing engine

`pricing.py` computes wholesale quotes. Published price policy (the source of truth):

1. Unit rates per SKU come from the per-warehouse rate tables (`rate_<category>_<region>`).
2. Volume discount on a line: 10% off for 100 units or more, 15% for 500 or more, 20% for 1000 or more.
3. Loyalty discount after the volume discount: 2% for customers of 3+ years, 5% for 10+ years.
4. Shipping: free when the goods subtotal is 500.00 or more; otherwise 12.50 for alpine and coastal, 7.50 elsewhere.
5. Regional tax applies to goods plus shipping.
"""

# ---------------------------------------------------------------- r2: ledger.py
VERBS = ["post", "settle", "accrue", "reconcile", "allocate", "defer", "amortize", "revalue", "net", "close",
         "open", "transfer", "book", "reverse", "adjust", "audit", "rollup", "split", "clear", "match"]
NOUNS = ["invoice", "receipt", "journal", "batch", "accrual", "credit", "debit", "payout", "refund", "fee",
         "rebate", "deposit", "advance", "voucher", "transfer", "balance", "reserve", "escrow", "ledger_line", "period"]


def r2_model():
    rng = random.Random(2602)
    names = rng.sample([f"{v}_{n}" for v in VERBS for n in NOUNS], 124)
    D = names[0:6]            # raise LedgerError directly
    T1 = names[6:11]          # call a D function, unprotected
    T2 = names[11:14]         # call a T1 function, unprotected
    W = names[14:15]          # calls a D function inside `except LedgerWarning` -> still propagates
    C = names[15:18]          # call D/T1 inside try/except LedgerError -> does not propagate
    CA = names[18:19]         # calls a D function inside `except Exception` -> does not propagate
    LW = names[19:21]         # raise LedgerWarning only (unrelated class)
    CM = names[21:23]         # mention LedgerError in comment/docstring only
    O = names[23:]            # ordinary
    # callable helpers: never reach a raiser, acyclic by rank. C and CA are top-level only (nobody calls them),
    # so the guarded calls into raisers can never form a cycle.
    safe = LW + CM + O
    rank = {n: i for i, n in enumerate(rng.sample(safe, len(safe)))}
    calls = {}
    for n in names:
        calls[n] = []
    for i, n in enumerate(D):
        calls[n] = []
    for n in T1:
        calls[n] = [rng.choice(D)]
    for n in T2:
        calls[n] = [rng.choice(T1)]
    plain_safe = {}
    for n in safe + D + T1 + T2 + W + C + CA:
        cands = [m for m in safe if m != n and (n not in rank or rank[m] < rank[n])]
        plain_safe[n] = rng.sample(cands, min(len(cands), rng.choice([0, 1, 1, 2])))
    guarded = {}
    for n in W:
        guarded[n] = ("LedgerWarning", rng.choice(D))
    for n in C:
        guarded[n] = ("LedgerError", rng.choice(D + T1))
    for n in CA:
        guarded[n] = ("Exception", rng.choice(D))
    expected = sorted(D + T1 + T2 + W)
    order = rng.sample(names, len(names))
    return dict(rng=rng, names=names, D=D, T1=T1, T2=T2, W=W, C=C, CA=CA, LW=LW, CM=CM, calls=calls,
                plain_safe=plain_safe, guarded=guarded, expected=expected, order=order)


def r2_source() -> str:
    m = r2_model()
    rng = random.Random(77)
    out = ['"""Double-entry ledger helpers.\n\nAll helpers take (entries, amount): a list of entry dicts and a running Decimal\namount, and return the adjusted amount.\n"""',
           "from decimal import Decimal", "", 'CENT = Decimal("0.01")', "", "",
           "class LedgerError(Exception):", '    """Raised when the books cannot be kept consistent."""', "", "",
           "class LedgerWarning(Exception):", '    """Raised for recoverable irregularities. Unrelated to LedgerError."""', "", ""]
    for n in m["order"]:
        verb, noun = n.split("_", 1)
        k = rng.randint(2, 9)
        doc = f"{verb.capitalize()} the {noun.replace('_', ' ')} entries against the running amount."
        if n in m["CM"]:
            doc += " Never raises LedgerError; callers may rely on that." if n == m["CM"][0] else ""
        L = [f"def {n}(entries, amount):", f'    """{doc}"""', '    total = Decimal("0")', "    for e in entries:",
             f'        value = Decimal(str(e.get("value", 0)))',
             f'        if e.get("kind") == "{rng.choice(["credit", "debit", "fee", "refund"])}":',
             f'            total += value * Decimal("1.0{k}")', "        else:", "            total -= value",
             "    adjusted = total + amount"]
        if n in m["CM"] and n == m["CM"][1]:
            L.append("    # A negative total used to raise LedgerError here; it is now clamped instead.")
        for c in m["calls"][n] + m["plain_safe"][n]:
            L.append(f"    adjusted = {c}(entries, adjusted)")
        if n in m["guarded"]:
            exc, c = m["guarded"][n]
            L += ["    try:", f"        adjusted = {c}(entries, adjusted)", f"    except {exc}:",
                  f'        adjusted = amount.quantize(CENT)']
        if n in m["D"]:
            th = rng.choice([1000, 5000, 25000, 100000])
            L += [f'    if abs(adjusted) > Decimal("{th}"):',
                  f'        raise LedgerError(f"{n}: imbalance {{adjusted}} exceeds {th}")']
        if n in m["LW"]:
            L += [f'    if adjusted == Decimal("0"):', f'        raise LedgerWarning("{n}: zero balance")']
        L += [f"    if adjusted < 0:", f"        adjusted = -adjusted / {rng.randint(2, 5)}",
              f"    return adjusted.quantize(CENT)", "", ""]
        # pad bodies so the file is ~2,500 lines: a short audit trail block per function
        pad = [f"    # audit: {n} step {i}: keep entry order stable for the period close" for i in range(rng.randint(3, 6))]
        L[2:2] = pad
        out.extend(L)
    return "\n".join(out).rstrip() + "\n"


def main():
    what = sys.argv[1]
    if what in ("r1", "r1-ref"):
        d = sys.argv[2]; os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "pricing.py"), "w").write(r1_source(buggy=(what == "r1")))
        if what == "r1":
            open(os.path.join(d, "README.md"), "w").write(R1_README)
    elif what == "r2":
        d = sys.argv[2]; os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "ledger.py"), "w").write(r2_source())
    elif what == "r2-expected":
        print("\n".join(r2_model()["expected"]))
    else:
        sys.exit("unknown fixture " + what)


if __name__ == "__main__":
    main()
