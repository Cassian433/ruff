# `os-path-splitext` (`PTH122`)

```toml
preview = true
lint.select = ["PTH122"]
```

## Extension

Indexing the result to get the extension is replaced with `Path.suffix`. The fix is unsafe because
`pathlib` normalizes the path first and, before Python 3.14, handles leading and trailing dots
differently.

```py
import os

os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:3:1
  |
3 | os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
  | ^^^^^^^^^^^^^^^^
help: Replace with `Path(...).suffix`
  |
1 | import os
2 + import pathlib
3 |
  - os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
4 + pathlib.Path("foo/bar.py").suffix  # snapshot: os-path-splitext
5 | os.path.splitext("foo/bar.py")[-1]  # snapshot: os-path-splitext
  |
note: This is an unsafe fix and may change runtime behavior
```

Negative indexing gets the same fix:

```py
os.path.splitext("foo/bar.py")[-1]  # snapshot: os-path-splitext
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:4:1
  |
4 | os.path.splitext("foo/bar.py")[-1]  # snapshot: os-path-splitext
  | ^^^^^^^^^^^^^^^^
help: Replace with `Path(...).suffix`
  |
1 | import os
2 + import pathlib
3 |
4 | os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
  - os.path.splitext("foo/bar.py")[-1]  # snapshot: os-path-splitext
5 + pathlib.Path("foo/bar.py").suffix  # snapshot: os-path-splitext
  |
note: This is an unsafe fix and may change runtime behavior
```

## `Path` arguments

A `Path` argument is used as-is instead of being wrapped in another `Path` call:

```py
import os
from pathlib import Path

os.path.splitext(Path("foo/bar.py"))[1]  # snapshot: os-path-splitext
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:4:1
  |
4 | os.path.splitext(Path("foo/bar.py"))[1]  # snapshot: os-path-splitext
  | ^^^^^^^^^^^^^^^^
help: Replace with `Path(...).suffix`
  |
3 |
  - os.path.splitext(Path("foo/bar.py"))[1]  # snapshot: os-path-splitext
4 + Path("foo/bar.py").suffix  # snapshot: os-path-splitext
  |
note: This is an unsafe fix and may change runtime behavior
```

## Other uses

The root and the full tuple have no direct `pathlib` equivalent, so these calls get a diagnostic
without a fix:

```py
import os

os.path.splitext("foo/bar.py")[0]  # snapshot: os-path-splitext
root, ext = os.path.splitext("foo/bar.py")  # error: [os-path-splitext]
os.path.splitext("foo/bar.py")  # error: [os-path-splitext]
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:3:1
  |
3 | os.path.splitext("foo/bar.py")[0]  # snapshot: os-path-splitext
  | ^^^^^^^^^^^^^^^^
```

The same applies when the argument is unpacked:

```py
def f(*args):
    os.path.splitext(*args)[1]  # snapshot: os-path-splitext
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:7:5
  |
7 |     os.path.splitext(*args)[1]  # snapshot: os-path-splitext
  |     ^^^^^^^^^^^^^^^^
```

## Without preview

```toml
preview = false
lint.select = ["PTH122"]
```

The fix is only available in preview:

```py
import os

os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
```

```snapshot
error[PTH122]: `os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`
 --> src/mdtest_snippet.py:3:1
  |
3 | os.path.splitext("foo/bar.py")[1]  # snapshot: os-path-splitext
  | ^^^^^^^^^^^^^^^^
help: Replace with `Path(...).suffix`
```
