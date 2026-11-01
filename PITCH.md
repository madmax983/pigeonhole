# Pigeonhole

**Sixteen thousand holes, more pigeons than that.** A test bed for
autumn-web 0.9's database fleet in **slot mode**.

In slot mode the fleet never has more than 16,384 SQLite files. A tenant's id
is hashed to a slot, the slot names the file, and any number of tenants share
it. The file count is bounded however many tenants arrive, and a slot is the
unit that would move between hosts. The cost is that tenants are no longer
separated by a file boundary; they are separated by a `WHERE tenant_id = ?`
that the generated repository adds to every query.

Pigeonhole makes both halves visible.

## The moment

You open the board, a 128 × 128 square of slots, almost all dark, and press
**Release a flock**. Three thousand pigeons land. The squares fill in teal,
and they start doubling up (violet, then amber) long before the board is
anything like full: by three thousand pigeons, ~250 files already hold two
or more. The panel prints the birthday-problem prediction beside the real
count, and they agree to within a percent.

Then you click a violet square, become one of its pigeons, and look at your
file: *9 rows in it, 3 yours.* Your neighbours' notes are in the same file,
and the repository will not show them to you. Click a neighbour, become them,
and look back.

Then the census reads every slot file on disk and checks every row against
the router. A row in the wrong file is a stray; the test suite plants one to
prove it would be caught.

## Where it could go

- Two hosts splitting the slot range (that is the next build, Baton).
- Slot rebalancing: move a slot range to a new file layout and re-run the
  census.
- A bigger flock, past the point where half the board is taken (~11,000).
