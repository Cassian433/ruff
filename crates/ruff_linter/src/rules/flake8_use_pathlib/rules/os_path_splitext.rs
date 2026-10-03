use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, Expr, ExprCall};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::codes::Category;
use crate::importer::ImportRequest;
use crate::preview::is_fix_os_path_splitext_enabled;
use crate::rules::flake8_use_pathlib::helpers::is_pathlib_path_call;
use crate::{Edit, Fix, FixAvailability, Violation};

/// ## What it does
/// Checks for uses of `os.path.splitext`.
///
/// ## Why is this bad?
/// `pathlib` offers a high-level API for path manipulation, as compared to
/// the lower-level API offered by `os.path`. When possible, using `Path` object
/// methods such as `Path.suffix` and `Path.stem` can improve readability over
/// the `os.path` module's counterparts (e.g., `os.path.splitext()`).
///
/// `os.path.splitext()` specifically returns a tuple of the file root and
/// extension (e.g., given `splitext('/foo/bar.py')`, `os.path.splitext()`
/// returns `("foo/bar", ".py")`. These outputs can be reconstructed through a
/// combination of `Path.suffix` (`".py"`), `Path.stem` (`"bar"`), and
/// `Path.parent` (`"foo"`).
///
/// ## Examples
/// ```python
/// import os
///
/// (root, ext) = os.path.splitext("foo/bar.py")
/// ```
///
/// Use instead:
/// ```python
/// from pathlib import Path
///
/// path = Path("foo/bar.py")
/// root = path.parent / path.stem
/// ext = path.suffix
/// ```
///
/// ## Known issues
/// While using `pathlib` can improve the readability and type safety of your code,
/// it can be less performant than the lower-level alternatives that work directly with strings,
/// especially on older versions of Python.
///
/// ## Fix Safety
/// A fix is only available when the call is immediately indexed to get the extension, as in
/// `os.path.splitext(path)[1]`, which is replaced with `Path(path).suffix`.
///
/// This fix is always marked as unsafe because `Path.suffix` does not always match the extension
/// returned by `os.path.splitext`:
///
/// - `pathlib` normalizes the path first, which removes trailing slashes (e.g.,
///   `os.path.splitext("a.py/")[1]` is `""`, but `Path("a.py/").suffix` is `".py"`).
/// - Before Python 3.14, `Path.suffix` handles leading and trailing dots differently (e.g.,
///   `os.path.splitext("foo.")[1]` is `"."`, but `Path("foo.").suffix` is `""`).
/// - `os.path.splitext` accepts `bytes` paths, while `Path` raises a `TypeError` for them.
///
/// ## References
/// - [Python documentation: `Path.suffix`](https://docs.python.org/3/library/pathlib.html#pathlib.PurePath.suffix)
/// - [Python documentation: `Path.suffixes`](https://docs.python.org/3/library/pathlib.html#pathlib.PurePath.suffixes)
/// - [Python documentation: `os.path.splitext`](https://docs.python.org/3/library/os.path.html#os.path.splitext)
/// - [PEP 428 – The pathlib module – object-oriented filesystem paths](https://peps.python.org/pep-0428/)
/// - [Correspondence between `os` and `pathlib`](https://docs.python.org/3/library/pathlib.html#corresponding-tools)
/// - [Why you should be using pathlib](https://treyhunner.com/2018/12/why-you-should-be-using-pathlib/)
/// - [No really, pathlib is great](https://treyhunner.com/2019/01/no-really-pathlib-is-great/)
#[derive(ViolationMetadata)]
#[violation_metadata(stable_since = "v0.0.231", category = Category::Pedantic)]
pub(crate) struct OsPathSplitext {
    /// Whether the call is indexed to get the extension, as in `os.path.splitext(path)[1]`.
    extension_only: bool,
}

impl Violation for OsPathSplitext {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`os.path.splitext()` should be replaced by `Path.suffix`, `Path.stem`, and `Path.parent`"
            .to_string()
    }

    fn fix_title(&self) -> Option<String> {
        self.extension_only
            .then(|| "Replace with `Path(...).suffix`".to_string())
    }
}

/// PTH122
pub(crate) fn os_path_splitext(checker: &Checker, call: &ExprCall, segments: &[&str]) {
    if segments != ["os", "path", "splitext"] {
        return;
    }

    // `os.path.splitext(p)` takes a single argument.
    let path = call
        .arguments
        .find_argument_value("p", 0)
        .filter(|_| call.arguments.len() == 1);

    // Only the extension has a direct `pathlib` equivalent, so we only offer a fix for calls like
    // `os.path.splitext(path)[1]`. The root, `os.path.splitext(path)[0]`, would need
    // `Path(path).with_suffix("")`, which returns a `Path` instead of a string and raises a
    // `ValueError` for paths with an empty name, such as `""` or `"/"`.
    let subscript = checker
        .semantic()
        .current_expression_parent()
        .and_then(Expr::as_subscript_expr)
        .filter(|subscript| {
            subscript.ctx.is_load()
                && subscript.value.range() == call.range()
                && is_extension_index(&subscript.slice)
        });

    let mut diagnostic = checker.report_diagnostic(
        OsPathSplitext {
            extension_only: path.is_some() && subscript.is_some(),
        },
        call.func.range(),
    );

    if !is_fix_os_path_splitext_enabled(checker.settings()) {
        return;
    }

    let (Some(path), Some(subscript)) = (path, subscript) else {
        return;
    };

    diagnostic.try_set_fix(|| {
        let (import_edit, binding) = checker.importer().get_or_import_symbol(
            &ImportRequest::import("pathlib", "Path"),
            call.start(),
            checker.semantic(),
        )?;

        let path_code = checker.locator().slice(path.range());
        let replacement = if is_pathlib_path_call(checker, path) {
            format!("{path_code}.suffix")
        } else {
            format!("{binding}({path_code}).suffix")
        };

        Ok(Fix::unsafe_edits(
            Edit::range_replacement(replacement, subscript.range()),
            [import_edit],
        ))
    });
}

/// Returns `true` if `expr` is `1` or `-1`, the index of the extension in the `(root, ext)` tuple
/// returned by `os.path.splitext`.
fn is_extension_index(expr: &Expr) -> bool {
    let operand = match expr {
        Expr::UnaryOp(ast::ExprUnaryOp {
            op: ast::UnaryOp::USub,
            operand,
            ..
        }) => operand,
        _ => expr,
    };

    matches!(
        operand,
        Expr::NumberLiteral(ast::ExprNumberLiteral {
            value: ast::Number::Int(int),
            ..
        }) if *int == ast::Int::ONE
    )
}
