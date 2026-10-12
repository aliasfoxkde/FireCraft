//! FireCraft scripting host: a sandboxed [`boa_engine`] embedding that
//! executes Fireworks-style `.jsf` scripts against a host object model
//! (`fw`, `dom`, `smartShape`, `Files`).
//!
//! The engine is the M1 decision (decision 0001): `boa_engine` for its
//! wasm build; the integration surface is the host contract, not the
//! JS engine. Mutating host calls do not touch the document from
//! inside the interpreter — they queue `(command id, payload)`
//! intents, and [`run_in_session`] drains the queue through
//! [`engine::Session::apply`] only when the run completed, so a
//! failing script leaves the document untouched and every applied
//! mutation rides the journal exactly like a GUI or CLI mutation.
//!
//! Layer position: L4 (may depend on `doc`/`engine`/`compose`; the
//! engine never depends on this crate).

use std::rc::Rc;

use boa_engine::{Context, JsResult, JsValue, Source, js_string, native_function::NativeFunction};
use boa_gc::{Gc, GcRefCell};

pub mod dom;
pub mod host;
pub mod marshal;
mod widget;

use doc::ObjectId;
use engine::{Delta, Session, delta::ObjectInverse, platform::FileSystem};

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

/// One queued mutation: a registry command id (or `host::control`
/// id) and its JSON/number payload in string form — the queue lives
/// inside the GC heap, so it holds `Trace` types only.
#[derive(Debug, Default, boa_gc::Finalize, boa_gc::Trace)]
struct Intent {
    id: String,
    payload: String,
}

/// Shared host state the native functions see: remaining call budget,
/// the note log, the queued intents, the revert mark, the host-side
/// selection (object ids as `"index:generation"`), and the current
/// page (view state; `changeCurrentPage`). `Gc` because boa native
/// closures capture `Trace` types (`from_copy_closure` needs `Copy`
/// captures; `from_closure` is `unsafe`, which the workspace forbids).
#[derive(Debug, Default, boa_gc::Finalize, boa_gc::Trace)]
struct HostState {
    budget: u32,
    notes: Vec<String>,
    intents: Vec<Intent>,
    selection: Vec<String>,
    current_page: u32,
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
            intents: Vec::new(),
            selection: Vec::new(),
            current_page: 0,
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
    /// `fw.getDocumentDOM()`, `fw.selection`, the mutating `dom`
    /// methods. A snapshot — re-attach after mutations so reads stay
    /// fresh. Replaces any previous projection.
    ///
    /// # Errors
    /// A string when projection or installation fails (internal
    /// defect; document data that cannot project is skipped in-place
    /// by the projection itself).
    pub fn attach_document(&mut self, document: &doc::Document) -> Result<(), String> {
        let (selection, current_page) = {
            let state = self.state.borrow();
            (state.selection.clone(), state.current_page)
        };
        dom::attach(
            &mut self.context,
            document,
            &dom::AttachView {
                selection,
                current_page: current_page as usize,
            },
        )
        .map_err(|error| format!("dom projection failed: {error}"))
    }

    /// Runs `source` (a `.jsf` body or fragment) to completion, a
    /// throw, or a limit. BOM/CRLF normalization is applied first —
    /// shipped `.jsf` files are UTF-8 BOM + CRLF. The report carries
    /// only this run's `__log` output (the note log grows across
    /// runs; reports are per-run).
    pub fn run(&mut self, name: &str, source: &str) -> ScriptReport {
        let prepared = prepare_source(source);
        let notes_before = self.state.borrow().notes.len();
        let result = self.context.eval(Source::from_bytes(prepared.as_bytes()));
        let host_calls = self.budget_total - self.state.borrow().budget;
        let notes: Vec<String> = self.state.borrow().notes[notes_before..].to_vec();
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

    /// Discards queued intents — the per-run state [`run_in_session`]
    /// starts from. Selection and the current page are cross-run
    /// state and survive.
    fn reset_run_state(&self) {
        self.state.borrow_mut().intents.clear();
    }

    /// Takes the queued intents, leaving the queue empty.
    fn take_intents(&self) -> Vec<Intent> {
        std::mem::take(&mut self.state.borrow_mut().intents)
    }

    /// Replaces the host-side selection.
    fn set_selection(&self, ids: Vec<String>) {
        self.state.borrow_mut().selection = ids;
    }
}

/// Runs `source` against `session`'s document: attach (fresh
/// projection), run, and — only when the run completed — drain the
/// queued mutations through [`Session::apply`] in queue order, then
/// re-attach so a follow-up run sees fresh state.
///
/// Semantics (the host-model mutation contract):
/// - a throwing or limit-aborted run queues mutations but never
///   applies them — a failing script leaves the document untouched;
/// - `fw.setUpRevert` marks the queue position; `fw.revert` discards
///   everything queued after the mark (nothing was applied yet, so
///   revert is exact — no undo residue in journal or history);
/// - a drain-time command failure (bad target after earlier commands
///   shifted the document) applies nothing further, keeps what did
///   apply, and reports [`Outcome::Threw`] — the script's declared
///   effects did not land.
pub fn run_in_session(
    session: &mut Session,
    engine: &mut ScriptEngine,
    name: &str,
    source: &str,
) -> ScriptReport {
    if let Err(error) = engine.attach_document(session.document()) {
        return ScriptReport {
            outcome: Outcome::Threw,
            message: Some(format!("{name}: {error}")),
            notes: Vec::new(),
            host_calls: 0,
        };
    }
    engine.reset_run_state();
    let mut report = engine.run(name, source);
    if report.completed() {
        drain(session, engine, &mut report);
    } else {
        let _dropped = engine.take_intents();
    }
    // Fresh projection either way: applied mutations show up, and the
    // host-side selection state replaces the script's own `fw.selection`
    // assignments (selection is view state; commands are truth).
    if let Err(error) = engine.attach_document(session.document()) {
        report.outcome = Outcome::Threw;
        report.message = Some(format!("{name}: post-run attach failed: {error}"));
    }
    report
}

/// Runs an instance's definition rich-symbol script: the `Widget`
/// re-derivation contract. `op_code` 1 runs the script in
/// set-defaults mode, 2 applies the instance's current values — the
/// two codes CS6's Component panel sends (`Widget.opCode`).
///
/// Semantics match [`run_in_session`] exactly: attach, project the
/// `Widget` global (`widget::project`), run the definition's script,
/// and — only on completion — drain the queued mutations through the
/// registry in queue order (decision 0011), then re-attach. A throw,
/// a limit, or a drain failure applies nothing further and reports
/// [`Outcome::Threw`]; what did apply rides the journal like any
/// command chain, so the whole chain undoes and replays byte-exactly.
///
/// The `Widget` global stays installed on the engine until the next
/// re-derivation replaces it; plain `run_in_session` scripts that
/// don't touch `Widget` are unaffected.
pub fn rederive_symbol(
    session: &mut Session,
    engine: &mut ScriptEngine,
    instance: ObjectId,
    op_code: u8,
) -> ScriptReport {
    const NAME: &str = "widget";
    if let Err(error) = engine.attach_document(session.document()) {
        return ScriptReport {
            outcome: Outcome::Threw,
            message: Some(format!("{NAME}: {error}")),
            notes: Vec::new(),
            host_calls: 0,
        };
    }
    engine.reset_run_state();
    let source = match widget::project(&mut engine.context, session.document(), instance, op_code) {
        Ok(source) => source,
        Err(refusal) => {
            let _dropped = engine.take_intents();
            return ScriptReport {
                outcome: Outcome::Threw,
                message: Some(format!("{NAME}: {refusal}")),
                notes: Vec::new(),
                host_calls: 0,
            };
        }
    };
    let mut report = engine.run(NAME, &source);
    if report.completed() {
        drain(session, engine, &mut report);
    } else {
        let _dropped = engine.take_intents();
    }
    if let Err(error) = engine.attach_document(session.document()) {
        report.outcome = Outcome::Threw;
        report.message = Some(format!("{NAME}: post-run attach failed: {error}"));
    }
    report
}

/// Applies the queued intents. Marks and reverts are resolved FIRST —
/// nothing has been applied yet, so revert is exact: the discarded
/// commands never touch document, journal, or history. Survivors go
/// through [`Session::apply_delta`] in queue order, so journal,
/// history, and dirty-flag bookkeeping are identical to a direct
/// invocation.
fn drain(session: &mut Session, engine: &mut ScriptEngine, report: &mut ScriptReport) {
    // Resolve marks/reverts; the page change is order-independent
    // (creations bake their page into each payload at queue time).
    let intents = engine.take_intents();
    let mut survivors: Vec<Intent> = Vec::new();
    let mut mark = 0usize;
    let mut page_change: Option<usize> = None;
    for intent in intents {
        match intent.id.as_str() {
            host::control::MARK => mark = survivors.len(),
            host::control::REVERT => {
                survivors.truncate(mark);
                engine.set_selection(Vec::new());
            }
            host::control::PAGE => page_change = Some(payload_usize(&intent.payload)),
            _ => survivors.push(intent),
        }
    }

    if let Some(target) = page_change {
        if target >= session.document().pages.len() {
            fail_drain(
                report,
                engine,
                &format!("changeCurrentPage: page {target} is not on this document"),
            );
            return;
        }
        engine.state.borrow_mut().current_page = target as u32;
    }

    for intent in survivors {
        let payload = match serde_json::from_str(&intent.payload) {
            Ok(payload) => payload,
            Err(error) => {
                fail_drain(
                    report,
                    engine,
                    &format!("{}: payload does not parse: {error}", intent.id),
                );
                return;
            }
        };
        match session.apply_delta(&intent.id, payload) {
            Ok(delta) => {
                let created = delta_created_ids(&delta);
                if !created.is_empty() {
                    engine.set_selection(created.iter().map(|id| id_key(*id)).collect());
                }
            }
            Err(error) => {
                fail_drain(report, engine, &format!("{}: {error}", intent.id));
                return;
            }
        }
    }
}

/// Fails the drain: keeps applied commands, drops the rest, reports
/// the failure.
fn fail_drain(report: &mut ScriptReport, engine: &mut ScriptEngine, message: &str) {
    let _dropped = engine.take_intents();
    report.outcome = Outcome::Threw;
    report.message = Some(message.to_owned());
}

/// `"index:generation"` — the string form a projected element carries
/// as `__fwId` and the host-side selection stores.
fn id_key(id: ObjectId) -> String {
    format!("{}:{}", id.index(), id.generation())
}

/// `usize` from a control payload (`{"page":n}` / bare integer JSON).
fn payload_usize(payload: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|value| match value {
            serde_json::Value::Number(number) => number.as_u64().map(|n| n as usize),
            serde_json::Value::Object(map) => map
                .values()
                .next()
                .and_then(|value| value.as_u64().map(|n| n as usize)),
            _ => None,
        })
        .unwrap_or(usize::MAX)
}

/// Collects the ids a delta created (the addNew\* semantics: what the
/// command made becomes the selection).
fn delta_created_ids(delta: &Delta) -> Vec<ObjectId> {
    match delta {
        Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        } => vec![*id],
        Delta::Group(parts) => parts.iter().flat_map(delta_created_ids).collect(),
        _ => Vec::new(),
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
/// reader, the `__fwQueue` mutation sink, and the JS half (host-array
/// Proxy + typed shells) from [`host::PRELUDE`].
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
            state.clone(),
        ),
    )?;

    // The mutation sink: every mutating host call funnels here with a
    // registry command id and its JSON payload. Applied only when the
    // run completes (see [`run_in_session`]).
    context.register_global_callable(
        js_string!("__fwQueue"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let id = args
                    .first()
                    .unwrap_or(&JsValue::undefined())
                    .to_string(ctx)?
                    .to_std_string_escaped();
                let payload = args
                    .get(1)
                    .unwrap_or(&JsValue::undefined())
                    .to_string(ctx)?
                    .to_std_string_escaped();
                state.borrow_mut().intents.push(Intent { id, payload });
                Ok(JsValue::undefined())
            },
            state.clone(),
        ),
    )?;

    // Script-file access: `Files.*` and `fw.runScript` resolve through
    // the attached [`FileSystem`] (thread-scoped, see
    // [`run_with_file_system`]). Each consumes budget.
    context.register_global_callable(
        js_string!("__fwRead"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let path = string_arg(args, ctx)?;
                let bytes = fs_op("Files.read", move |fs| fs.read_file(&path))?;
                let text = String::from_utf8_lossy(&bytes);
                Ok(JsValue::from(js_string!(text.as_ref())))
            },
            state.clone(),
        ),
    )?;

    context.register_global_callable(
        js_string!("__fwWrite"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let path = string_arg(args, ctx)?;
                let contents = args
                    .get(1)
                    .unwrap_or(&JsValue::undefined())
                    .to_string(ctx)?
                    .to_std_string_escaped();
                fs_op("Files.write", move |fs| {
                    fs.write_file(&path, contents.as_bytes())
                })?;
                Ok(JsValue::from(true))
            },
            state.clone(),
        ),
    )?;

    context.register_global_callable(
        js_string!("__fwExists"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let path = string_arg(args, ctx)?;
                let exists = current_file_system()?.exists(&path);
                Ok(JsValue::from(exists))
            },
            state.clone(),
        ),
    )?;

    // `fw.runScript(path)`: loads and evaluates immediately, so the
    // sub-script queues its mutations into the same intent queue and
    // reads the same snapshot the caller sees. Recursion is bounded by
    // the runtime limits + the host budget like any script.
    context.register_global_callable(
        js_string!("__fwRunScript"),
        1,
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, state: &Gc<GcRefCell<HostState>>, ctx| {
                take_budget(state)?;
                let path = string_arg(args, ctx)?;
                let bytes = fs_op("fw.runScript", move |fs| fs.read_file(&path))?;
                let source = prepare_source(&String::from_utf8_lossy(&bytes));
                ctx.eval(Source::from_bytes(source.as_bytes()))?;
                Ok(JsValue::undefined())
            },
            state,
        ),
    )?;

    // The prelude's JS half: host-array Proxy + the typed shells.
    context.eval(Source::from_bytes(host::PRELUDE))?;
    Ok(())
}

/// The one string argument, as a Rust string.
fn string_arg(args: &[JsValue], ctx: &mut Context) -> JsResult<String> {
    Ok(args
        .first()
        .unwrap_or(&JsValue::undefined())
        .to_string(ctx)?
        .to_std_string_escaped())
}

/// The script file system for the current thread ([`run_with_file_system`]
/// scoped). Missing or failed attachments throw typed, never silently.
fn current_file_system() -> JsResult<Rc<dyn FileSystem>> {
    FILE_SYSTEM
        .with(|slot| slot.borrow().clone())
        .ok_or_else(|| {
            boa_engine::JsNativeError::error()
                .with_message("no file system is attached to this script run")
                .into()
        })
}

/// Runs a [`FileSystem`] operation, mapping engine errors into typed
/// JS errors that name the calling host operation.
fn fs_op<T>(
    name: &str,
    op: impl FnOnce(&dyn FileSystem) -> Result<T, engine::EngineError>,
) -> JsResult<T> {
    let fs = current_file_system()?;
    op(fs.as_ref()).map_err(|error| {
        boa_engine::JsNativeError::error()
            .with_message(format!("{name}: {error}"))
            .into()
    })
}

thread_local! {
    /// The attached script file system. Boa's [`Context`] is
    /// single-threaded, and the attachment is always cleared when
    /// [`run_with_file_system`] returns, so engines cannot leak file
    /// systems into each other.
    static FILE_SYSTEM: std::cell::RefCell<Option<Rc<dyn FileSystem>>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs `f` with `fs` attached as the script file system — what
/// `Files.*` and `fw.runScript` resolve through. The attachment is
/// thread-scoped and cleared when this returns; a run without one
/// gets typed "no file system" errors from those calls.
pub fn run_with_file_system<R>(fs: Rc<dyn FileSystem>, f: impl FnOnce() -> R) -> R {
    FILE_SYSTEM.with(|slot| *slot.borrow_mut() = Some(fs));
    let result = f();
    FILE_SYSTEM.with(|slot| *slot.borrow_mut() = None);
    result
}
