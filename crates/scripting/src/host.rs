//! The host prelude's JS half: the host-array Proxy (the decision-0001
//! semantic — reading an unassigned index below `length` materializes
//! a default element, which is how shipped scripts grow point lists),
//! the typed `fw`/`dom` shells, and the `smartShape` global.
//!
//! Kept as a JS source string so the prelude is executable grammar,
//! not a mirror of it. Names marked "not implemented" throw a typed
//! error naming the M9 task that owns them — visible in reports,
//! never silently wrong.

/// The JS prelude source, evaluated once per [`ScriptEngine`].
pub const PRELUDE: &str = r#"
// --- host-array: Fireworks' grow-by-read list semantic ---------------
// `__hostArray(make)` returns a Proxy over `{length: n}`; reading an
// integer index below `length` that was never assigned materializes
// `make(index)` and stores it. `list[list.length++] = {...}` assigns
// normally; `list[3]` on a grown-but-unassigned list auto-fills.
globalThis.__hostArray = function (make) {
    var target = { length: 0 };
    return new Proxy(target, {
        get: function (t, k) {
            if (k === "length") { return t.length; }
            var i = Number(k);
            if (Number.isInteger(i) && i >= 0 && i < t.length && !(k in t)) {
                t[k] = make(i);
            }
            return t[k];
        },
        set: function (t, k, v) { t[k] = v; return true; },
    });
};

// --- typed refusal ----------------------------------------------------
globalThis.__notImplemented = function (name, task) {
    throw new Error(
        "not implemented in FireCraft: " + name + " (M9 task " + task + ")"
    );
};

// --- smartShape: the Auto Shape handler global ------------------------
// `operation` is set by the host before dispatch; handlers read it.
globalThis.smartShape = {
    operation: "",
    // Set by the host to a shape descriptor when a handler runs.
    GetCurrentContour: function () {
        __notImplemented("smartShape.GetCurrentContour", 3);
    },
    EndDragSmartShape: function () {
        __notImplemented("smartShape.EndDragSmartShape", 3);
    },
    RegisterDragInsert: function (insert) {
        __notImplemented("smartShape.RegisterDragInsert", 3);
    },
};

// --- fw: the top-level scripting API ----------------------------------
globalThis.fw = {
    // Host logging (native `__fwLog` behind a friendly name).
    __log: function () {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) { parts.push(String(arguments[i])); }
        __fwLog(parts.join(" "));
    },
    getDocumentDOM: function () {
        __notImplemented("fw.getDocumentDOM", 2);
    },
    setUpRevert: function () {
        __notImplemented("fw.setUpRevert", 3);
    },
    revert: function () {
        __notImplemented("fw.revert", 3);
    },
    runScript: function (path) {
        __notImplemented("fw.runScript", 4);
    },
    runCommands: function () {
        __notImplemented("fw.runCommands", 4);
    },
    dispatchEvent: function (event) {
        __notImplemented("fw.dispatchEvent", 3);
    },
    // Remaining native-call budget (sandbox observability).
    __budgetLeft: function () {
        return __fwBudgetRemaining();
    },
};

// --- dom: the document-DOM object shell --------------------------------
// The real projection over `doc::Document` lands in task 2; scripts
// that touch it now get the typed refusal.
globalThis.dom = {
    addNewRectangle: function () {
        __notImplemented("dom.addNewRectangle", 3);
    },
};

// --- Files: script file access (real FS lands in task 4) --------------
globalThis.Files = {
    read: function (path) {
        __notImplemented("Files.read", 4);
    },
    write: function (path, contents) {
        __notImplemented("Files.write", 4);
    },
    exists: function (path) {
        __notImplemented("Files.exists", 4);
    },
};
"#;
