# brayne
Brayne is a self-hosted command line timed repetition tool for flashcard memorization - in Rust

## Design

### On use of a Ledger backstore

A ledger system was chosen for Brayne because it made for a simple on-disk storage algorithm with incremental update, avoiding complexities of random read/write behavior that is common for mutable datastructures backed to disk.  The disadvantage is in vulnerability to corruption in a larger run-length file.

For a card memorization application, ledger use is of reduced risk relative to other applications.  This is because the behavior of Brayne gracefully degrades with loss of ledger data.  Specifically, the card next attempt timing is minimally impacted by historical attempts prior to the last card failure - and has small variance with loss of attempts since last card failure.

## Supermemo Algorithm

* See SM-2 documentation at [supermemo.com](https://www.supermemo.com/english/ol/sm2.htm)

## Alternatives Considered

### CRDTs

Considered support for CRDT underlying all Ledger entries.  This would make e.g. offline use easy to synchronize when returning to connectivity and needing to synchronise with other clients.

### On Handling Time

Ultimately, the AttemptRecord must contain a timestamp of the attempt (in order to compute when the card should next be exposed), and we have no option but to use the local time source - as there is no server-side to this implementation.  The weakness here is many fold.  A user can have incorrect system time, attempt some cards, and record timestamps in the future (which upon correcting system time, *could* result in the cards not being exposed).  Or the incorrect system time could be slow (e.g. 1970) and upon correction all cards are up for challenge - even while they should not be.  A few heristics may be applied to ease

## Ledger Fixup

There seem to be a few possible causes of Ledger issues:

* Corruption to ledger
* Time offsets due to incorrect system time during ledger writes

A tool to fixup ledger issues would be helpful

## TODO

* Fixup [error handling](https://doc.rust-lang.org/book/first-edition/error-handling.html)

## Potential extentions

### Randomized Control Trial

For a single user, cards could be assigned randomly to one of several interval algorithms - which persist for the life of the card.  This would enable A:B:..:N comparison of various repeat interval algorithms.

