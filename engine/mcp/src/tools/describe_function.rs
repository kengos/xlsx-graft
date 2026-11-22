//! The `describe_function` tool: looks a worksheet function up in the
//! compatibility table `verify` uses, through `xlsx_graft_host`, the same
//! JSON the CLI gives.

mod schema;

use super::{Tool, arguments, structured};
use crate::Context;
use crate::rpc::{RpcError, clip};
use serde_json::Value;
use xlsx_graft_host::describe_function;

/// The `describe_function` row of [`super::TOOLS`].
pub(super) const TOOL: Tool = Tool {
    name: "describe_function",
    title: "Describe a function's support",
    description: "Looks the function `name` up in the compatibility table that verify uses, \
                  and returns the table's entries for it: the name without a leading \
                  `_xlfn.` and `_xlws.`, in upper case; the name Excel writes for it in a \
                  file (`fileName`) and its `prefix`; the version of each application that \
                  added it; the features it needs as well; its safety; and the sources. \
                  Returns { name, known, fileName, prefix, support, requires, safety, \
                  sources }: `known` is `versioned`, `baseline` (no `support` and no \
                  `requires`) or `unknown` (only `name` and `known`; verify denies the \
                  name). A name the table does not list is not an error. The `name` in the \
                  result is truncated to 200 characters, with control characters removed.",
    input_schema: schema::input,
    output_schema: Some(schema::output),
    run,
    page: "describe-function",
};

/// Describes the function `arguments.name` names.
///
/// A call whose `arguments` are not an object with a string `name`, or hold
/// another key, is a protocol error (`INVALID_PARAMS`). Every name, one the
/// table does not list included, is answered with `isError: false`. The
/// `name` of the result is truncated and has its control characters removed,
/// in `structuredContent` and in the text, which is the same JSON.
fn run(arguments: &Value, _context: &Context) -> Result<Value, RpcError> {
    let arguments = arguments::read(arguments, "describe_function", &["name"])?;
    // The description's JSON has string keys only, so this does not fail;
    // `Null` is only the value the type needs for a case that cannot happen.
    let description = describe_function(arguments.required);
    let mut value = serde_json::to_value(&description).unwrap_or(Value::Null);
    // Assigning to `value["name"]` does not panic on an object or on `Null`
    // (`serde_json` turns `Null` into an object); the description's JSON
    // always holds `name`.
    value["name"] = clip(description.name()).into();
    Ok(structured(&value))
}
