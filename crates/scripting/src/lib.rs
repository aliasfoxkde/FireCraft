//! FireCraft scripting host: a sandboxed [`boa_engine`] embedding that
//! executes Fireworks-style `.jsf` scripts against a host object model
//! (`fw`, `dom`, `smartShape`, `Files`).
//!
//! The engine is the M1 decision (decision 0001): `boa_engine` for its
//! wasm build; the integration surface is the host contract, not the
//! JS engine. This crate is the sandbox + host prelude; DOM projection
//! and command dispatch land in later M9 tasks — host names that do
//! not exist yet throw a typed "not implemented" so scripts fail
//! loudly, never silently.
//!
//! Layer position: L4 (may depend on `doc`/`engine`/`compose`; the
//! engine never depends on this crate).

use boa_engine::{Context, JsResult, JsValue, Source, js_string, native_function::NativeFunction};
use boa_gc::{Gc, GcRefCell};

pub mod dom;
pub mod host;
pub mod marshal;

/// Sandbox limits. Defaults are generous for Auto Shape handlers
/// (point loops in the hundreds) and hostile for `while (true)`.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum loop iterations before the interpreter interrupts.
    pub loop_iterations: u64,
    /// Maximum recursive native/JS calls.
    pub recursion: usize,
    /// JS stack size in bytes.
    pub stack_bytes: usize,
    /// Native host-function invocations per script run — bounds
    /// `while (true) { <host call> }` even if each iteration is cheap.
    pub host_calls: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            loop_iterations: 1_000_000,
            recursion: 512,
            stack_bytes: 1 << 20,
            host_calls: 10_000,
        }
    }
}

/// Why a script run ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Outcome {
    /// The script ran to completion.
    #[default]
    Completed,
    /// The script threw (syntax error, host refusal, user error).
    Threw,
    /// A sandbox limit fired (loop, recursion, stack, host budget).
    LimitExceeded,
}

/// What one script run did. Never a panic: every failure mode lands
/// here.
#[derive(Clone, Debug, Default)]
pub struct ScriptReport {
    pub outcome: Outcome,
    /// Throw message or limit description, when not completed.
    pub message: Option<String>,
    /// Host `__log` output, in call order.
    pub notes: Vec<String>,
    /// Host-function invocations consumed by this run.
    pub host_calls: u32,
}

impl ScriptReport {
    /// True when the script ran to completion.
    pub fn completed(&self) -> bool {
        self.outcome == Outcome::Completed
    }
}

/// Shared host state the native functions see: remaining call budget
/// and the note log. `Gc` because boa native closures capture `Trace`
/// types (`from_copy_closure` needs `Copy` captures; `from_closure` is
/// `unsafe`, which the workspace forbids).
#[derive(Debug, Default, boa_gc::Finalize, boa_gc::Trace)]
struct HostState {
    budget: u32,
    notes: Vec<String>,
}

/// A prepared scripting context: sandbox limits applied, host prelude
/// installed. Reusable — run many scripts on one engine (the
/// long-running-session posture decision 0001 asks to benchmark).
pub struct ScriptEngine {
    context: Context,
    state: Gc<GcRefCell<HostState>>,
    budget_total: u32,
}

impl ScriptEngine {
    /// Builds an engine with `limits` and the host prelude installed.
    ///
    /// # Errors
    /// A string when the prelude itself fails to install — that is an
    /// internal defect, never script input.
    pub fn new(limits: Limits) -> Result<Self, String> {
        let mut context = Context::default();
        let runtime = context.runtime_limits_mut();
        runtime.set_loop_iteration_limit(limits.loop_iterations);
        runtime.set_recursion_limit(limits.recursion);
        runtime.set_stack_size_limit(limits.stack_bytes);

        let state = Gc::new(GcRefCell::new(HostState {
            budget: limits.host_calls,
            notes: Vec::new(),
        }));
        install_prelude(&mut context, state.clone())
            .map_err(|error| format!("host prelude install failed: {error}"))?;

        Ok(Self {
            context,
            state,
            budget_total: limits.host_calls,
        })
    }

    /// Projects `document` into the read surface: `fw.documents`,
    /// `fw.getDocumentDOM()`, `fw.selection`. A snapshot — re-attach
    /// after mutations so reads stay fresh. Replaces any previous
    /// projection.
    ///
    /// # Errors
    /// A string when projection or installation fails (internal
    /// defect; document data that cannot project is skipped in-place
    /// by the projection itself).
    pub fn attach_document(&mut self, document: &doc::Document) -> Result<(), String> {
        dom::attach(&mut self.context, document)
            .map_err(|error| format!("dom projection failed: {error}"))
    }

    /// Runs `source` (a `.jsf` body or fragment) to completion, a
    /// throw, or a limit. BOM/CRLF normalization is applied first —
    /// shipped `.jsf` files are UTF-8 BOM + CRLF.
    pub fn run(&mut self, name: &str, source: &str) -> ScriptReport {
        let prepared = prepare_source(source);
        let result = self.context.eval(Source::from_bytes(prepared.as_bytes()));
        let host_calls = self.budget_total - self.state.borrow().budget;
        let notes = self.state.borrow().notes.clone();
        match result {
            Ok(_) => ScriptReport {
                outcome: Outcome::Completed,
                message: None,
                notes,
                host_calls,
            },
            Err(error) => {
                let text = error.to_string();
                let outcome = if text.starts_with("RuntimeLimit") || text.contains(BUDGET_EXCEEDED)
                {
                    Outcome::LimitExceeded
                } else {
                    Outcome::Threw
                };
                ScriptReport {
                    outcome,
                    message: Some(format!("{name}: {text}")),
                    notes,
                    host_calls,
                }
            }
        }
    }
}

/// Marker the budget native throws on; [`ScriptEngine::run`]
/// classifies it as [`Outcome::LimitExceeded`].
const BUDGET_EXCEEDED: &str = "host call budget exhausted";

/// Strips a UTF-8 BOM and normalizes CRLF to LF (shipped-script
/// normalization, pinned by test).
pub fn prepare_source(source: &str) -> String {
    let no_bom = source.strip_prefix('\u{feff}').unwrap_or(source);
    no_bom.replace("\r\n", "\n")
}

/// Consumes one unit of host budget, refusing when exhausted. Every
/// native host function calls this first.
fn take_budget(state: &Gc<GcRefCell<HostState>>) -> JsResult<()> {
    let mut state = state.borrow_mut();
    state.budget = state.budget.saturating_sub(1);
    if state.budget == 0 {
        return Err(boa_engine::JsNativeError::error()
            .with_message(BUDGET_EXCEEDED)
            .into());
    }
    Ok(())
}

/// Installs the host prelude: the `__fwLog` native, the `__fwBudget`
/// reader, and the JS half (host-array Proxy + typed shells) from
/// [`host::PRELUDE`].
fn install_prelude(context: &mut Context, state: Gc<GcRefCell<HostState>>) -> JsResult<()> {
    context.register_global_callable(
        js_string!("__fwLog"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let mut parts = String::new();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        parts.push(' ');
                    }
                    parts.push_str(&arg.to_string(ctx)?.to_std_string_escaped());
                }
                state.borrow_mut().notes.push(parts);
                Ok(JsValue::undefined())
            },
            state.clone(),
        ),
    )?;

    context.register_global_callable(
        js_string!("__fwBudgetRemaining"),
        0,
        NativeFunction::from_copy_closure_with_captures(
            |_this, _args, state: &Gc<GcRefCell<HostState>>, _ctx| {
                Ok(JsValue::from(state.borrow().budget))
            },
            state,
        ),
    )?;

    // The prelude's JS half: host-array Proxy + the typed shells.
    context.eval(Source::from_bytes(host::PRELUDE))?;
    Ok(())
}
