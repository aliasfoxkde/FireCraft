//! The host prelude's JS half: the host-array Proxy (the decision-0001
//! semantic — reading an unassigned index below `length` materializes
//! a default element, which is how shipped scripts grow point lists),
//! the typed `fw`/`dom` shells, the `smartShape` global, script-file
//! access, and the per-attach mutating methods ([`DOM_MUTATORS`]).
//!
//! Kept as JS source strings so the prelude is executable grammar,
//! not a mirror of it. Names marked "not implemented" throw a typed
//! error naming the M9 task that owns them — visible in reports,
//! never silently wrong.

/// Control-intent ids the drain interprets ([`crate::run_in_session`]
/// handles them; they never reach the registry).
pub mod control {
    /// `fw.setUpRevert` — the drain notes the queue position.
    pub const MARK: &str = "__fw.mark";
    /// `fw.revert` — the drain undoes everything applied since the
    /// mark and clears selection.
    pub const REVERT: &str = "__fw.revert";
    /// `dom.changeCurrentPage` — validated and persisted as view
    /// state (no document delta).
    pub const PAGE: &str = "__fw.page";
    /// `dom.setShowGrid` — view state, no document delta (the wire
    /// has no node; decision 0035).
    pub const SHOW_GRID: &str = "__fw.showGrid";
    /// `dom.setShowGuides` — view state (decision 0035).
    pub const SHOW_GUIDES: &str = "__fw.showGuides";
    /// `dom.setSnapToGrid` — view state (decision 0035).
    pub const SNAP_TO_GRID: &str = "__fw.snapToGrid";
    /// `dom.setSnapToGuides` — view state (decision 0035).
    pub const SNAP_TO_GUIDES: &str = "__fw.snapToGuides";
}

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
        return __fwRunScript(String(path));
    },
    // Corpus shape: command names as strings. Each queues with an empty
    // payload — commands whose arguments are all optional succeed at
    // drain, the rest fail typed (name visible in the report).
    runCommands: function () {
        for (var i = 0; i < arguments.length; i++) {
            if (typeof arguments[i] !== "string") {
                throw new Error(
                    "runCommands: argument " + i + " must be a command name string"
                );
            }
            __fwQueue(arguments[i], "{}");
        }
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

// --- Files: script file access ------------------------------------------
// read/write/exists resolve through the natives (`__fwRead`/`__fwWrite`/
// `__fwExists`), which speak the `engine::platform::FileSystem` attached
// for this run — virtual FS on web, app/user dirs on desktop. A run
// with no attached FS gets a typed error, never a silent default.
globalThis.Files = {
    read: function (path) {
        return __fwRead(String(path));
    },
    write: function (path, contents) {
        return __fwWrite(String(path), contents === undefined ? "" : String(contents));
    },
    exists: function (path) {
        return __fwExists(String(path));
    },
    // Names the shipped corpus touches beyond read/write/exists; task 5
    // (the corpus run harness) owns wiring the ones its scripts need.
    getLanguageDirectory: function () {
        __notImplemented("Files.getLanguageDirectory", 5);
    },
    deleteFileIfExisting: function (path) {
        __notImplemented("Files.deleteFileIfExisting", 5);
    },
    createDirectory: function (path) {
        __notImplemented("Files.createDirectory", 5);
    },
    open: function (path, mode) {
        __notImplemented("Files.open", 5);
    },
    createFile: function (path) {
        __notImplemented("Files.createFile", 5);
    },
    getDirectory: function (prompt, forOpen) {
        __notImplemented("Files.getDirectory", 5);
    },
    enumFiles: function (path) {
        __notImplemented("Files.enumFiles", 5);
    },
    getFilename: function (prompt, save, defaultName) {
        __notImplemented("Files.getFilename", 5);
    },
    copy: function (from, to) {
        __notImplemented("Files.copy", 5);
    },
};
"#;

/// The mutating host methods, evaluated once per attach on the fresh
/// projection. Every mutation funnels through `__fwQueue(command id,
/// payload JSON)` — the drain applies queued commands only when the
/// run completes, so a failing script leaves the document untouched
/// and applied mutations ride the journal like any GUI mutation.
///
/// Signatures follow the shipped corpus (`addNewLayer(null, false)`,
/// `moveSelectionToLayer(i, false, "none", -1)`); arguments the model
/// does not carry yet are accepted and ignored rather than refused.
pub const DOM_MUTATORS: &str = r#"
// --- argument guards ---------------------------------------------------
function __requireNumbers(name, values) {
    for (var i = 0; i < values.length; i++) {
        if (typeof values[i] !== "number" || !isFinite(values[i])) {
            throw new Error(name + ": argument " + i + " must be a finite number");
        }
    }
}
function __requireInt(name, value) {
    if (typeof value !== "number" || !isFinite(value) || Math.floor(value) !== value) {
        throw new Error(name + ": expected an integer, got " + value);
    }
}
function __selectedIds(name) {
    var ids = [];
    for (var i = 0; i < fw.selection.length; i++) {
        var id = fw.selection[i] && fw.selection[i].__fwId;
        if (!id || typeof id !== "object" || typeof id.index !== "number") {
            throw new Error(name + ": selection entry " + i + " is not a projected element");
        }
        ids.push(id);
    }
    return ids;
}

// --- element creation (addNew* selects what it created) ----------------
__fwDom.addNewText = function (x, y, width, height, text, name) {
    __requireNumbers("addNewText", [x, y, width, height]);
    __fwQueue("text.createText", JSON.stringify({
        page: __fwDom.currentPageNum,
        layer: __fwDom.currentLayerNum,
        cell: __fwDom.currentFrameNum,
        x: x, y: y, width: width, height: height,
        text: (text === undefined || text === null) ? "" : String(text),
        name: (name === undefined || name === null) ? null : String(name),
    }));
};

__fwDom.addNewRectangle = function (x, y, width, height, name) {
    __requireNumbers("addNewRectangle", [x, y, width, height]);
    __fwQueue("object.createPath", JSON.stringify({
        kind: "rect",
        x: x, y: y, width: width, height: height,
        page: __fwDom.currentPageNum,
        layer: __fwDom.currentLayerNum,
        cell: __fwDom.currentFrameNum,
        name: (name === undefined || name === null) ? null : String(name),
    }));
};

// --- layers -------------------------------------------------------------
// Corpus shape: addNewLayer(null, false) — the second argument is
// Fireworks' group-type flag, which has no FireCraft meaning yet.
__fwDom.addNewLayer = function (name, groupFlag) {
    var index = __fwDom.layers.length;
    __fwQueue("layer.add", JSON.stringify({
        page: __fwDom.currentPageNum,
        name: (name === undefined || name === null) ? null : String(name),
        index: null,
    }));
    // New layers become active immediately (view state; the drain's
    // re-attach recomputes the projection with the layer present).
    __fwDom.currentLayerNum = index;
};

__fwDom.deleteLayer = function (index) {
    __requireInt("deleteLayer", index);
    __fwQueue("layer.delete", JSON.stringify({
        page: __fwDom.currentPageNum,
        layer: index,
    }));
};

// --- selection (commands are truth; the view mirrors immediately) --------
// The model carries the object selection (decision 0024). Each verb
// queues its command (applied at drain, undoable) AND resolves against
// the current projection so mid-run reads agree with the queued truth.
// `fw.selection = [...]` assignments stay plain view state (documented
// deviation; the command surface is the durable path).
function __selectionMirrorLayer(index, frameNum) {
    var layer = __fwDom.layers[index];
    if (!layer) {
        throw new Error("selectAllOnLayer: layer " + index + " is not on this page");
    }
    var frame = layer.frames[frameNum] || layer.frames[0];
    return frame ? frame.elements.slice() : [];
}

__fwDom.selectAll = function () {
    __requireInt("selectAll", __fwDom.currentFrameNum);
    __fwQueue("selection.selectAll", JSON.stringify({
        page: __fwDom.currentPageNum,
        frame: __fwDom.currentFrameNum,
    }));
    var all = [];
    for (var i = 0; i < __fwDom.layers.length; i++) {
        all = all.concat(__selectionMirrorLayer(i, __fwDom.currentFrameNum));
    }
    fw.selection = all;
};

__fwDom.selectAllOnLayer = function (index) {
    var i = (index === undefined) ? __fwDom.currentLayerNum : index;
    __requireInt("selectAllOnLayer", i);
    __fwQueue("selection.selectAllOnLayer", JSON.stringify({
        page: __fwDom.currentPageNum,
        layer: i,
        frame: __fwDom.currentFrameNum,
        remember_selection: false,
        toggle_selection: false,
    }));
    fw.selection = __selectionMirrorLayer(i, __fwDom.currentFrameNum);
};

__fwDom.selectInverse = function () {
    __fwQueue("selection.selectInverse", JSON.stringify({
        page: __fwDom.currentPageNum,
        frame: __fwDom.currentFrameNum,
    }));
    var selected = {};
    for (var i = 0; i < fw.selection.length; i++) {
        var id = fw.selection[i] && fw.selection[i].__fwId;
        if (id) {
            selected[id.index + ":" + id.generation] = true;
        }
    }
    var inverse = [];
    for (var i = 0; i < __fwDom.layers.length; i++) {
        var candidates = __selectionMirrorLayer(i, __fwDom.currentFrameNum);
        for (var j = 0; j < candidates.length; j++) {
            var id = candidates[j] && candidates[j].__fwId;
            if (id && !selected[id.index + ":" + id.generation]) {
                inverse.push(candidates[j]);
            }
        }
    }
    fw.selection = inverse;
};

__fwDom.selectNone = function () {
    __fwQueue("selection.selectNone", "{}");
    fw.selection = [];
};

// --- selection-carrying mutations ---------------------------------------
// Corpus shape: moveSelectionToLayer(layerIndex, replaceCurrentLayer,
// findKind, findParameter); the trailing three have no FireCraft
// meaning yet and are accepted and ignored.
__fwDom.moveSelectionToLayer = function (layerIndex) {
    __requireInt("moveSelectionToLayer", layerIndex);
    var ids = __selectedIds("moveSelectionToLayer");
    if (ids.length === 0) {
        throw new Error("moveSelectionToLayer: nothing is selected");
    }
    for (var i = 0; i < ids.length; i++) {
        __fwQueue("object.moveToCell", JSON.stringify({
            id: ids[i],
            page: __fwDom.currentPageNum,
            layer: layerIndex,
            cell: __fwDom.currentFrameNum,
            index: null,
        }));
    }
};

// Union of every selected path, folded pairwise (a keeps and absorbs;
// the fold is deterministic in selection order).
__fwDom.pathUnion = function () {
    var ids = __selectedIds("pathUnion");
    if (ids.length < 2) {
        throw new Error("pathUnion: select the paths to combine first");
    }
    for (var i = 1; i < ids.length; i++) {
        __fwQueue("path.boolean", JSON.stringify({
            a: ids[0],
            b: ids[i],
            op: "union",
        }));
    }
};

// Corpus shape: applies to the single selected text object.
__fwDom.setTextAutoExpand = function (auto) {
    if (fw.selection.length !== 1) {
        throw new Error("setTextAutoExpand: select exactly one text object");
    }
    __fwQueue("text.setAttrs", JSON.stringify({
        id: __selectedIds("setTextAutoExpand")[0],
        auto_enlarge: !!auto,
    }));
};

// --- view state -----------------------------------------------------------
__fwDom.changeCurrentPage = function (index) {
    __requireInt("changeCurrentPage", index);
    if (index < 0 || index >= __fwDom.pagesCount) {
        throw new Error("changeCurrentPage: page " + index + " is not on this document");
    }
    __fwDom.currentPageNum = index;
    // Persisted so the next attach projects the same page.
    __fwQueue("__fw.page", String(index));
};

// Canvas view flags (decision 0035): session state — the four verbs
// queue a control intent and write the projection in place, so a read
// in the same run agrees; no document delta is ever queued.
function __requireBoolean(name, value) {
    if (typeof value !== "boolean") {
        throw new Error(name + ": expected a boolean, got " + typeof value);
    }
}
function __viewFlag(queueId, propName, fnName, value) {
    __requireBoolean(fnName, value);
    __fwDom[propName] = value;
    __fwQueue(queueId, String(value));
}
__fwDom.setShowGrid = function (bShow) {
    __viewFlag("__fw.showGrid", "showGrid", "setShowGrid", bShow);
};
__fwDom.setShowGuides = function (bShow) {
    __viewFlag("__fw.showGuides", "showGuides", "setShowGuides", bShow);
};
__fwDom.setSnapToGrid = function (bSnap) {
    __viewFlag("__fw.snapToGrid", "snapToGrid", "setSnapToGrid", bSnap);
};
__fwDom.setSnapToGuides = function (bSnap) {
    __viewFlag("__fw.snapToGuides", "snapToGuides", "setSnapToGuides", bSnap);
};

function __isMasterPageLayer() {
    return __fwDom.currentPageNum === __fwDom.masterPageNum;
}
__fwDom.isMasterPageLayer = function (index) {
    __requireInt("isMasterPageLayer", index);
    return __isMasterPageLayer();
};

// --- revert ---------------------------------------------------------------
fw.setUpRevert = function () {
    __fwQueue("__fw.mark", "0");
};
fw.revert = function () {
    __fwQueue("__fw.revert", "0");
};
"#;
