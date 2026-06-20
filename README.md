# fasta-tui

A terminal viewer for FASTA DNA sequences, built with [Ratatui](https://ratatui.rs).
It pairs a color-coded sequence browser with an animated, depth-shaded Braille
double helix, plus composition statistics and live amino-acid translation — all
inside your terminal.

```
┌ 1:Reader  2:Stats  3:Protein ─────────────────────  seq1 [1/3] ┐
│ ╭ GC Content — synthetic construct ──────────────╮  ╭ 3D Double Helix ╮
│ │ 54.0%  (180 bases)  ████████████░░░░░░░░░░░░░░ │  │      A⠂⠐T        │
│ ╰────────────────────────────────────────────────╯  │     ⠈C   G⠠      │
│ ╭ Sequence (A·T·G·C colored, ATG=start … ) ──────╮▲  │    G⠐     T⠂     │
│ │      1 │ ATGGCATTAG CCGGGTACCA TGGATCCGGT …    ┃  │     ⠈T   A⠠      │
│ │     61 │ ATGAAACCCG GGTTTTAAGC GCGCATATGC …    ┃  │      C⠂⠐G        │
│ ╰────────────────────────────────────────────────╯▼  ╰─────────────────╯
│  FWD   base A @ 1/180    F+0   Neon   SPIN        ←/→ base  ↑/↓ row  ?:help q:quit
└────────────────────────────────────────────────────────────────────────┘
```

## Features

- **Sequence reader** — a genome-browser-style view with a position gutter,
  10-base groups, and per-nucleotide coloring (A/T/G/C each get their own hue).
  Start codons (`ATG`) and stop codons (`TAA`/`TAG`/`TGA`) are highlighted in a
  non-overlapping left-to-right scan, and the base under the cursor is marked.
- **3D double helix** — a Braille-rendered, rotating double helix on the right
  pane. Front strands are drawn bright over dimmed back strands for a sense of
  depth, with the actual base letters floating on the rungs. It spins on idle and
  rotates as you scroll; speed and twist are adjustable.
- **Stats view** — a base-composition bar chart and a GC-content sliding-window
  line chart (computed with prefix sums for O(1) window queries).
- **Protein view** — live translation of the selected strand and reading frame
  using the standard genetic code, with start (`M`) and stop (`*`) residues
  highlighted.
- **Motif search** — incremental `A/T/G/C` motif search across the current
  record, with match highlighting and jump-to-next/previous navigation.
- **Reverse complement & reading frames** — flip to the 3'→5' strand and cycle
  reading frames 0/1/2; the helix, reader, and translation all follow.
- **Multi-record files** — open multi-FASTA files and tab between records.
- **Flexible input** — plain or gzip-compressed FASTA, reading from a file or
  standard input (`-`), with RNA (`U`) and IUPAC ambiguity codes accepted.
- **Protein export** — `--export-protein` translates every record to a protein
  FASTA file (or stdout) headlessly, for use in scripts and pipelines.
- **Themes** — three built-in color palettes (Neon, Pale, High-Contrast).

## Installation

Requires a [Rust toolchain](https://rustup.rs/) (edition 2021).

```bash
git clone <repo-url>
cd fasta-tui
cargo build --release
# the binary lands at target/release/fasta-tui
```

To install it onto your `PATH`:

```bash
cargo install --path .
```

## Usage

Pass the path to a FASTA file as the only argument:

```bash
cargo run --release -- samples/synthetic.fa
# or, if installed:
fasta-tui samples/synthetic.fa

# gzip files work as-is, and `-` reads from standard input:
fasta-tui sequence.fasta.gz
curl -s https://example.org/genome.fa | fasta-tui -
```

A [`samples/`](samples/) folder with ready-to-use FASTA files — including a real
GenBank record — is included to try it out. See [`samples/README.md`](samples/README.md).

Supported input:

- `.fasta` / `.fa` text files with one or more `>` header records.
- Gzip-compressed input, detected by magic bytes (the `.gz` extension is optional).
- Standard input via `-`, so `fasta-tui` slots into Unix pipelines.
- Header-less raw sequence files (gathered into a single anonymous record).
- Lower-case letters, whitespace, and line wrapping are all fine — the parser
  uppercases input, reads RNA `U` as `T`, keeps IUPAC ambiguity codes
  (`N`, `R`, `Y`, …), and ignores gaps and digits.

### Exporting protein translations

Translate every record to protein and write a FASTA file without opening the
TUI — handy for scripting:

```bash
# forward strand, reading frame 0, to a file
fasta-tui sequence.fasta --export-protein proteins.faa

# reverse strand in frame 1, straight to stdout
fasta-tui sequence.fasta --export-protein - --reverse --frame 1
```

## Key bindings

Press `?` at any time to toggle the in-app key reference.

### Navigation

| Key | Action |
| --- | --- |
| `←` / `h`, `→` / `l` | Move the cursor one base left / right |
| `↑` / `k`, `↓` / `j` | Move the cursor one row up / down |
| `PageUp` / `PageDown` | Move one page |
| `Home` / `g`, `End` / `G` | Jump to the start / end of the record |
| `Tab` / `]` | Next FASTA record |
| `BackTab` / `[` | Previous FASTA record |

### Views

| Key | Action |
| --- | --- |
| `1` / `2` / `3` | Switch view: Reader / Stats / Protein |
| `r` | Toggle reverse-complement strand |
| `f` | Cycle reading frame 0 / 1 / 2 |
| `t` | Cycle color theme |

### Search

| Key | Action |
| --- | --- |
| `/` | Start a motif search (type `A`/`T`/`G`/`C`, `Enter` to confirm, `Esc` to cancel) |
| `n` / `N` | Jump to the next / previous match |

### Helix controls

| Key | Action |
| --- | --- |
| `Space` | Pause / resume the idle spin |
| `+` / `-` | Spin faster / slower |
| `>` / `<` | More / less twist |

### General

| Key | Action |
| --- | --- |
| `?` | Toggle the help overlay |
| `q` / `Esc` | Quit |

## Status bar

The bottom line summarizes the current state as color-coded chips:

- `FWD` / `REV` — current strand orientation.
- `base X @ pos/len` — the nucleotide under the cursor and its 1-based position.
- `F+n` — the active reading frame.
- theme name and `SPIN` / `PAUSED` for the helix.
- `/motif  i/total` — appears while a search has matches.

## Project layout

```
src/
├── main.rs            # entry point + event loop (polls input, ticks animation)
├── cli.rs             # command-line argument parsing (clap)
├── theme.rs           # color palettes shared across widgets
├── fasta/             # domain layer (no UI dependencies)
│   ├── parse.rs       # multi-record FASTA parser
│   ├── record.rs      # Record model, complement / reverse-complement
│   ├── stats.rs       # base counts, GC content, sliding-window GC
│   └── translate.rs   # codon → amino-acid translation (standard code)
├── app/               # application state + input handling
│   ├── mod.rs         # App state, navigation, search, toggles
│   └── input.rs       # key-event → state mutation mapping
└── ui/                # rendering layer (Ratatui)
    ├── mod.rs         # top-level layout, tab bar, status line
    ├── reader.rs      # sequence reader + GC gauge + scrollbar
    ├── helix.rs       # the 3D Braille double helix
    ├── stats.rs       # composition + GC charts
    ├── protein.rs     # amino-acid translation view
    └── help.rs        # key-binding overlay
```

The codebase keeps a clean separation: the `fasta` module is pure domain logic
with unit tests and no terminal dependencies, `app` owns mutable state and input,
and `ui` is a stateless rendering layer that reads `&App` each frame.

## Development

```bash
cargo build      # compile
cargo test       # run the unit tests (parser, stats, translation)
cargo clippy     # lint
cargo run -- samples/synthetic.fa
```

The render loop polls for input on an 80 ms timeout and advances the helix
animation each tick, so the visualization keeps spinning even while idle.

## Dependencies

- [`ratatui`](https://crates.io/crates/ratatui) — terminal UI framework
- [`crossterm`](https://crates.io/crates/crossterm) — terminal backend / input
- [`clap`](https://crates.io/crates/clap) — command-line argument parsing

## License

See the repository for license details.
