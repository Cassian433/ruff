# `glob` (`PTH207`)

```toml
lint.select = ["PTH207"]
```

## Basic usage

`Path.glob()` yields `Path` objects instead of strings, includes hidden files, and always treats
`**` as recursive, so the suggested fix is display-only.

```py
import glob

glob.glob("*.py")  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:3:1
  |
3 | glob.glob("*.py")  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
1 | import glob
2 + import pathlib
3 |
  - glob.glob("*.py")  # snapshot: glob
4 + pathlib.Path().glob("*.py")  # snapshot: glob
5 | glob.iglob("*.py")  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

`glob.iglob()` gets the same suggestion:

```py
glob.iglob("*.py")  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `iglob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:4:1
  |
4 | glob.iglob("*.py")  # snapshot: glob
  | ^^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
1 | import glob
2 + import pathlib
3 |
4 | glob.glob("*.py")  # snapshot: glob
  - glob.iglob("*.py")  # snapshot: glob
5 + pathlib.Path().glob("*.py")  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

## Patterns built with `os.path.join`

`Path.glob()` raises an error for absolute patterns, so the directory arguments of `os.path.join()`
move into the `Path` constructor:

```py
import glob
import os

glob.glob(os.path.join(os.path.dirname(__file__), "data", "*.json"))  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:4:1
  |
4 | glob.glob(os.path.join(os.path.dirname(__file__), "data", "*.json"))  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
2 | import os
3 + import pathlib
4 |
  - glob.glob(os.path.join(os.path.dirname(__file__), "data", "*.json"))  # snapshot: glob
5 + pathlib.Path(os.path.dirname(__file__), "data").glob("*.json")  # snapshot: glob
6 | glob.glob(os.path.join("src", "**", "*.py"), recursive=True)  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

A wildcard in a directory argument can't be moved into the `Path` constructor, so the whole pattern
is kept:

```py
glob.glob(os.path.join("src", "**", "*.py"), recursive=True)  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:5:1
  |
5 | glob.glob(os.path.join("src", "**", "*.py"), recursive=True)  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
2 | import os
3 + import pathlib
4 |
5 | glob.glob(os.path.join(os.path.dirname(__file__), "data", "*.json"))  # snapshot: glob
  - glob.glob(os.path.join("src", "**", "*.py"), recursive=True)  # snapshot: glob
6 + pathlib.Path().glob(os.path.join("src", "**", "*.py"))  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

## `recursive`

`recursive` is dropped because `**` always matches recursively in `Path.glob()`:

```py
import glob

glob.glob("**/*.py", recursive=True)  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:3:1
  |
3 | glob.glob("**/*.py", recursive=True)  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
1 | import glob
2 + import pathlib
3 |
  - glob.glob("**/*.py", recursive=True)  # snapshot: glob
4 + pathlib.Path().glob("**/*.py")  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

## Existing `Path` import

An existing `Path` import is reused:

```py
import glob
from pathlib import Path

glob.glob("*.py")  # snapshot: glob
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:4:1
  |
4 | glob.glob("*.py")  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
  |
3 |
  - glob.glob("*.py")  # snapshot: glob
4 + Path().glob("*.py")  # snapshot: glob
  |
note: This is a display-only fix and is likely to be incorrect
```

## Arguments without a `pathlib` equivalent

`Path.glob()` has no `root_dir` or `include_hidden` parameters, so these calls get a diagnostic
without a fix:

```py
import glob

glob.glob("*.py", root_dir="src")  # snapshot: glob
glob.glob("*.py", include_hidden=True)  # error: [glob]
```

```snapshot
error[PTH207]: Replace `glob` with `Path.glob` or `Path.rglob`
 --> src/mdtest_snippet.py:3:1
  |
3 | glob.glob("*.py", root_dir="src")  # snapshot: glob
  | ^^^^^^^^^
help: Replace with `Path(...).glob(...)`
```

The same applies when the arguments are unpacked:

```py
def f(*args, **kwargs):
    glob.glob(*args)  # error: [glob]
    glob.iglob(**kwargs)  # error: [glob]
```

## `dir_fd`

`pathlib` doesn't support `dir_fd`, so calls that pass it are not reported:

```py
import glob

glob.glob("*.py", dir_fd=3)  # no diagnostic
```
