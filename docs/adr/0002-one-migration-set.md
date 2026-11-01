# ADR 0002 — One migration set, applied to control and to every slot file

Status: accepted (a workaround), 2026-10-31

## Context

Pigeonhole has tenant data (`notes`, belongs in each slot file) and
control-plane data (`pigeons`, the registry, belongs in the control database
only). autumn-web 0.9 applies
every app migration set registered with `AppBuilder::migrations` to the control
database **and** to every fleet database when it opens; only the framework's
own control-plane sets are kept off the fleet. There is no way to mark an app
set control-only or fleet-only.

## Decision

Register one set with both tables. Each slot file therefore carries an empty
`pigeons` table, and the control database an empty `notes` table.

## Alternatives considered

- Create `pigeons` by hand on the control pool in the startup hook. Works,
  but takes the table out of the migration history.

## Revisit when

autumn-web grows a way to scope an app migration set (for example
`.migrations_for(Control, …)` / `.migrations_for(Fleet, …)`). Then split the
set and drop the empty copies.
