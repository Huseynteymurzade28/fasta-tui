# Sample FASTA files

Ready-to-use inputs for `fasta-tui`. Run any of them with:

```bash
cargo run --release -- samples/<file>
# or, if installed:
fasta-tui samples/<file>
```

| File | What it is | Good for showing off |
| --- | --- | --- |
| `synthetic.fa` | 3-record synthetic multi-FASTA (start/stop codons, GC-rich, AT-rich) | Tabbing between records, codon highlighting |
| `maize-zein-X55661.fasta` | Real GenBank record — *Zea mays* mRNA for 22 kD zein protein ([X55661.1](https://www.ncbi.nlm.nih.gov/nuccore/X55661.1)) | A genuine gene: protein translation, real GC profile |
| `codons-demo.fasta` | Synthetic ORFs with clean `ATG…TAA`, many start codons, and all three stops | Reading-frame view, start/stop highlighting |
| `gc-extremes.fasta` | GC-rich, AT-rich, and balanced fragments | The Stats view GC-content sliding-window chart |
| `rna-and-ambiguous.fasta` | RNA (`U`) and IUPAC ambiguity codes (`N`, `R`, `Y`, …) | RNA→DNA reading, ambiguity-code coloring |
| `raw-no-header.txt` | Header-less raw sequence (no `>` line) | The "anonymous record" parser path |

`fasta-tui` also reads gzip-compressed files (`fasta-tui some.fasta.gz`) and
standard input (`cat file.fasta | fasta-tui -`), so you don't need a plain,
decompressed copy on disk.

All synthetic sequences are made up for demonstration; `maize-zein-X55661.fasta`
is a real public record from NCBI/GenBank.
