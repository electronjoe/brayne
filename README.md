# brayne
Brayne is a self-hosted command line timed repetition tool for flashcard memorization - in Rust

## Supermemo Algorithm

* See SM-2 documentation at [supermemo.com](https://www.supermemo.com/english/ol/sm2.htm)

## CRDTs

Considered support for CRDT underlying all Ledger entries.  This would make e.g. offline use easy to synchronize when returning to connectivity and needing to synchronise with other clients.

## On Handling Time

Ultimately, the AttemptRecord must contain a timestamp of the attempt (in order to compute when the card should next be exposed), and we have no option but to use the local time source - as there is no server-side to this implementation.  The weakness here is many fold.  A user can have incorrect system time, attempt some cards, and record timestamps in the future (which upon correcting system time, *could* result in the cards not being exposed).  Or the incorrect system time could be slow (e.g. 1970) and upon correction all cards are up for challenge - even while they should not be.  A few heristics may be applied to ease

## Ledger Fixup

There seem to be a few possible causes of Ledger issues:

* Corruption to ledger
* Time offsets due to incorrect system time during ledger writes

A tool to fixup ledger issues would be helpful
