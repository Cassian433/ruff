use ruff_diagnostics::Applicability;
use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::token::parenthesized_range;
use ruff_python_ast::{Expr, ExprCall};
use ruff_text_size::{Ranged, TextRange};

use crate::checkers::ast::Checker;
use crate::codes::Category;
use crate::importer::ImportRequest;
use crate::rules::flake8_use_pathlib::helpers::{
    has_unknown_keywords_or_starred_expr, is_keyword_only_argument_non_default,
};
use crate::{Edit, Fix, FixAvailability, Violation};

/// ## What it does
/// Checks for the use of `glob.glob()` and `glob.iglob()`.
///
/// ## Why is this bad?
/// `pathlib` offers a high-level API for path manipulation, as compared to
/// the lower-level API offered by `os` and `glob`.
///
/// When possible, using `Path` object methods such as `Path.glob()` can
/// improve readability over their low-level counterparts (e.g.,
/// `glob.glob()`).
///
/// Note that `glob.glob()` and `Path.glob()` are not exact equivalents:
///
/// |                   | `glob`-module functions                                                                                                                              | `Path.glob()`                                                                                                                                |
/// |-------------------|------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
/// | Hidden files      | Hidden files are excluded by default. On Python 3.11+, the `include_hidden` keyword can be used to include hidden directories.                       | Includes hidden files by default.                                                                                                            |
/// | Eagerness         | `glob.iglob()` returns a lazy iterator. Under the hood, `glob.glob()` simply converts the iterator to a list.                                        | `Path.glob()` returns a lazy iterator.                                                                                                       |
/// | Working directory | `glob.glob()` and `glob.iglob()` take a `root_dir` keyword to set the current working directory.                                                     | `Path.rglob()` can be used to return the relative path.                                                                                      |
/// | Globstar (`**`)   | The `recursive` flag must be set to `True` for the `**` pattern to match any files and zero or more directories, subdirectories, and symbolic links. | The `**` pattern in `Path.glob()` means "this directory and all subdirectories, recursively". In other words, it enables recursive globbing. |
///
/// ## Example
/// ```python
/// import glob
/// import os
///
/// glob.glob(os.path.join("my_path", "requirements*.txt"))
/// ```
///
/// Use instead:
/// ```python
/// from pathlib import Path
///
/// Path("my_path").glob("requirements*.txt")
/// ```
///
/// ## Known issues
/// While using `pathlib` can improve the readability and type safety of your code,
/// it can be less performant than the lower-level alternatives that work directly with strings,
/// especially on older versions of Python.
///
/// ## Fix Safety
/// This rule's fix is display-only: it is shown as a suggestion but never applied automatically,
/// because `Path.glob()` is not a drop-in replacement for `glob.glob()` or `glob.iglob()`. In
/// addition to the differences listed above, `Path.glob()` yields `Path` objects rather than
/// strings, and it raises an error for absolute patterns.
///
/// When the pattern is built with `os.path.join()`, as in the example above, the fix moves the
/// directory into the `Path` constructor so that absolute directories keep working.
///
/// No fix is offered if the `root_dir` or `include_hidden` keyword arguments are passed, since
/// `Path.glob()` has no equivalent for them. The `recursive` keyword argument is dropped, because
/// the `**` pattern always matches recursively in `Path.glob()`.
///
/// ## References
/// - [Python documentation: `Path.glob`](https://docs.python.org/3/library/pathlib.html#pathlib.Path.glob)
/// - [Python documentation: `Path.rglob`](https://docs.python.org/3/library/pathlib.html#pathlib.Path.rglob)
/// - [Python documentation: `glob.glob`](https://docs.python.org/3/library/glob.html#glob.glob)
/// - [Python documentation: `glob.iglob`](https://docs.python.org/3/library/glob.html#glob.iglob)
/// - [PEP 428 – The pathlib module – object-oriented filesystem paths](https://peps.python.org/pep-0428/)
/// - [Why you should be using pathlib](https://treyhunner.com/2018/12/why-you-should-be-using-pathlib/)
/// - [No really, pathlib is great](https://treyhunner.com/2019/01/no-really-pathlib-is-great/)
#[derive(ViolationMetadata)]
#[violation_metadata(stable_since = "v0.0.281", category = Category::Pedantic)]
pub(crate) struct Glob {
    pub function: String,
}

impl Violation for Glob {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        let Glob { function } = self;
        format!("Replace `{function}` with `Path.glob` or `Path.rglob`")
    }

    fn fix_title(&self) -> Option<String> {
        Some("Replace with `Path(...).glob(...)`".to_string())
    }
}

/// PTH207
pub(crate) fn glob(checker: &Checker, call: &ExprCall, segments: &[&str]) {
    let ["glob", function @ ("glob" | "iglob")] = segments else {
        return;
    };

    // `dir_fd` is not supported by pathlib, so check if it's set to non-default values.
    // Signature as of Python 3.13 (https://docs.python.org/3/library/glob.html#glob.glob)
    // ```text
    //               0           1              2            3                 4
    // glob.glob(pathname, *, root_dir=None, dir_fd=None, recursive=False, include_hidden=False)
    // ```
    if is_keyword_only_argument_non_default(&call.arguments, "dir_fd") {
        return;
    }

    let mut diagnostic = checker.report_diagnostic(
        Glob {
            function: (*function).to_string(),
        },
        call.func.range(),
    );

    // `Path.glob` has no equivalent for `root_dir` or `include_hidden`. `recursive` can be dropped
    // because `**` always matches recursively in `Path.glob`.
    if has_unknown_keywords_or_starred_expr(&call.arguments, &["pathname", "recursive"]) {
        return;
    }

    let Some(pattern) = call.arguments.find_argument_value("pathname", 0) else {
        return;
    };

    diagnostic.try_set_fix(|| {
        let (import_edit, binding) = checker.importer().get_or_import_symbol(
            &ImportRequest::import("pathlib", "Path"),
            call.start(),
            checker.semantic(),
        )?;

        let replacement = match split_os_path_join(checker, pattern) {
            Some((directory, pattern)) => format!("{binding}({directory}).glob({pattern})"),
            None => format!("{binding}().glob({})", checker.locator().slice(pattern)),
        };

        Ok(Fix::applicable_edits(
            Edit::range_replacement(replacement, call.range()),
            [import_edit],
            Applicability::DisplayOnly,
        ))
    });
}

/// If `pattern` is an `os.path.join()` call whose last argument is the only one that may contain
/// wildcards, returns the source code of the directory arguments and of the last argument.
///
/// `Path.glob` raises an error for absolute patterns, so a call like
/// `glob.glob(os.path.join(directory, "*.py"))` has to become `Path(directory).glob("*.py")`
/// rather than `Path().glob(os.path.join(directory, "*.py"))`.
fn split_os_path_join<'a>(checker: &'a Checker, pattern: &Expr) -> Option<(&'a str, &'a str)> {
    let join = pattern.as_call_expr()?;

    if !checker
        .semantic()
        .resolve_qualified_name(&join.func)
        .is_some_and(|name| matches!(name.segments(), ["os", "path", "join"]))
    {
        return None;
    }

    if !join.arguments.keywords.is_empty() || join.arguments.args.iter().any(Expr::is_starred_expr)
    {
        return None;
    }

    let [directories @ .., last] = &*join.arguments.args else {
        return None;
    };
    let (first, last_directory) = (directories.first()?, directories.last()?);

    // A wildcard in a directory argument, as in `os.path.join(root, "**", "*.py")`, can't be moved
    // into the `Path` constructor.
    if directories.iter().any(|directory| {
        directory.as_string_literal_expr().is_some_and(|string| {
            string
                .value
                .chars()
                .any(|char| matches!(char, '*' | '?' | '['))
        })
    }) {
        return None;
    }

    let source_range = |expr: &Expr| {
        parenthesized_range(expr.into(), (&join.arguments).into(), checker.tokens())
            .unwrap_or(expr.range())
    };

    let locator = checker.locator();
    let directory = locator.slice(TextRange::new(
        source_range(first).start(),
        source_range(last_directory).end(),
    ));
    let pattern = locator.slice(source_range(last));

    Some((directory, pattern))
}
