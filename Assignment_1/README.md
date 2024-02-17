# Files

- `structs.cpp`: Main file. Contains the implementation of both algorithms. Due to inheritance and templates, it was much cleaner to write both in one file.
- `VC-cs21btech11001.cpp`: Contains the `main` function for the VC algorithm.
- `SK-cs21btech11001.cpp`: Contains the `main` function for the SK algorithm.
- `inp-params.txt`: Input file.
- `plot.py`: Python script to plot the graph.
- `report.md`, `Assgn1-Report-cs21btech11001.pdf`: Report files.
- `run.sh`: Script to compile and run the programs.
- Various output and log files.

# Instructions to run

```bash
bash run.sh
```

This script calls `plt.plot()`, so the output is a window containing the graph. To recompile the report, run:

```bash
pandoc report.md -o Assgn1-Report-cs21btech11001.pdf
```