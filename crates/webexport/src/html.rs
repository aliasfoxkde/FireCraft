//! The HTML+Images writer: the SLICES.XTT shim-table structure —
//! doctype, head with the MM_* runtime when behaviors exist, the
//! `<table>` with shim edges and merged cells, `<map>`/`<area>`
//! emission for hotspots. HTML 4 variant, no comments (deterministic
//! bytes).

use crate::images::Piece;
use crate::layout::Grid;
use crate::naming;
use crate::{WebExportOptions, WebObject};

/// HTML-escapes attribute/text content.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// The behavior hooks one object's `<a>`/`<area>` carries, plus the
/// files its behaviors preload: `(onMouseOver, onMouseOut, onClick,
/// preloads)`. Kinds follow the template's constants — 2 swap image,
/// 4 button down, 5 swap restore, 6 button highlight, 7 button
/// restore. The swap/down file rides `DTAv` custom data under
/// `swapTarget` (decision 0017); a behavior without a target emits no
/// hook. Popup menus (9) and unobserved kinds stay unrendered (M11
/// non-goal): no hook, payload preserved in the file.
fn hooks(
    name: &str,
    slice: &doc::SliceObject,
) -> (Option<String>, Option<String>, Option<String>, Vec<String>) {
    let mut over = String::new();
    let mut out = String::new();
    let mut click = String::new();
    let mut preload = Vec::new();
    let target = slice
        .custom_data
        .get("swapTarget")
        .map(String::as_str)
        .filter(|target| !target.is_empty());
    for behavior in &slice.behaviors {
        match behavior.kind {
            // Nav-bar highlight: over.
            6 => {
                if let Some(target) = target {
                    over.push_str(&format!("MM_nbGroup('over','{name}','{target}','',1);"));
                    preload.push(target.to_owned());
                }
            }
            // Nav-bar down: click.
            4 => {
                if let Some(target) = target {
                    click.push_str(&format!(
                        "MM_nbGroup('down','navbar1','{name}','{target}',1);"
                    ));
                    preload.push(target.to_owned());
                }
            }
            // Swap image: over, restore on out.
            2 => {
                if let Some(target) = target {
                    over.push_str(&format!("MM_swapImage('{name}','','{target}',1);"));
                    preload.push(target.to_owned());
                    out.push_str("MM_swapImgRestore();");
                }
            }
            // Swap-image restore / button restore: out.
            5 => out.push_str("MM_swapImgRestore();"),
            7 => out.push_str("MM_nbGroup('out');"),
            _ => {}
        }
    }
    let over = (!over.is_empty()).then_some(over);
    let out = (!out.is_empty()).then_some(out);
    let click = (!click.is_empty()).then_some(click);
    (over, out, click, preload)
}

/// The clean-room MM runtime, emitted only when some hook needs it.
/// Functional equivalents of the Dreamweaver runtime's call
/// signatures, not Adobe's text.
const MM_RUNTIME: &str = r##"function MM_swapImgRestore() { //v3.0
  var i,x,a=document.MM_sr; for(i=0;a&&i<a.length&&(x=a[i])&&x.oSrc;i++) x.src=x.oSrc;
}
function MM_preloadImages() { //v3.0
  var d=document; if(d.images){ if(!d.MM_p) d.MM_p=new Array();
    var i,j=d.MM_p.length,a=MM_preloadImages.arguments; for(i=0; i<a.length; i++)
    if (a[i].indexOf("#")!=0){ d.MM_p[j]=new Image; d.MM_p[j++].src=a[i]; }}
}
function MM_findObj(n, d) { //v4.01
  var p,i,x;  if(!d) d=document; if((p=n.indexOf("?"))>0&&parent.frames.length) {
    d=parent.frames[n.substring(p+1)].document; n=n.substring(0,p);}
  if(!(x=d[n])&&d.all) x=d.all[n]; for (i=0;!x&&i<d.forms.length;i++) x=d.forms[i][n];
  for(i=0;!x&&d.layers&&i<d.layers.length;i++) x=MM_findObj(n,d.layers[i].document);
  if(!x&&d.getElementById) x=d.getElementById(n); return x;
}
function MM_swapImage() { //v3.0
  var i,j=0,x,a=MM_swapImage.arguments; document.MM_sr=new Array; for(i=0;i<(a.length-2);i+=3)
   if ((x=MM_findObj(a[i]))!=null){document.MM_sr[j++]=x; if(!x.oSrc) x.oSrc=x.src; x.src=a[i+2];}
}
function MM_nbGroup(event, name, file) { // nav-bar state machine
  if (event == "init" && name != "") { return; }
  var img = (name != "") ? MM_findObj(name) : null;
  if (img && (event == "over" || event == "down")) {
    if (!img.oSrc) img.oSrc = img.src;
    if (event == "down") document.MM_nbDown = img;
    img.src = file;
  }
  if (event == "out" && document.MM_nbDown) {
    document.MM_nbDown.src = document.MM_nbDown.oSrc; document.MM_nbDown = null;
  }
}
"##;

/// The canvas background as `#rrggbb` (raw is ABGR).
fn bg_hex(raw: i64) -> String {
    let raw = raw as u32;
    format!(
        "#{:02x}{:02x}{:02x}",
        raw & 0xFF,
        (raw >> 8) & 0xFF,
        (raw >> 16) & 0xFF
    )
}

/// A hotspot `<area>`'s `(shape, coords)` relative to the host
/// piece's origin.
fn area_geometry(slice: &doc::SliceObject, origin_x: u32, origin_y: u32) -> (&'static str, String) {
    let frame = &slice.frame;
    let (ox, oy) = (f64::from(origin_x), f64::from(origin_y));
    match slice.shape {
        1 => {
            let (cx, cy) = (
                (frame.x0 + frame.x1) / 2.0 - ox,
                (frame.y0 + frame.y1) / 2.0 - oy,
            );
            let radius = (frame.x1 - frame.x0).max(frame.y1 - frame.y0) / 2.0;
            ("circle", format!("{cx:.0},{cy:.0},{radius:.0}"))
        }
        2 => {
            let mut coords = String::new();
            for contour in &slice.contours {
                for point in &contour.points {
                    if !coords.is_empty() {
                        coords.push(',');
                    }
                    coords.push_str(&format!(
                        "{:.0},{:.0}",
                        point.anchor.x - ox,
                        point.anchor.y - oy
                    ));
                }
            }
            ("poly", coords)
        }
        _ => (
            "rect",
            format!(
                "{:.0},{:.0},{:.0},{:.0}",
                frame.x0 - ox,
                frame.y0 - oy,
                frame.x1 - ox,
                frame.y1 - oy
            ),
        ),
    }
}

/// A hotspot anchored to the piece under its center.
struct Anchor {
    /// Piece index the area rides.
    piece: usize,
    /// Web-object index of the hotspot.
    hotspot: usize,
    /// Hook attributes `(over, out, click)` for the area.
    over: Option<String>,
    out: Option<String>,
    click: Option<String>,
}

/// The cell index holding coordinate `at`, given the cut list.
fn cut_index(cuts: &[u32], at: f64) -> Option<usize> {
    if cuts.len() < 2 {
        return None;
    }
    (0..cuts.len() - 1)
        .find(|&index| f64::from(cuts[index]) <= at && at < f64::from(cuts[index + 1]))
}

/// Anchors every hotspot (a `TSLi`-less web object) to the piece
/// containing its center, in object order.
fn anchor_hotspots(objects: &[WebObject<'_>], grid: &Grid, pieces: &[Piece]) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        if object.slice.slice_type.is_some() {
            continue; // slices raster; hotspots map
        }
        let center_x = (object.slice.frame.x0 + object.slice.frame.x1) / 2.0;
        let center_y = (object.slice.frame.y0 + object.slice.frame.y1) / 2.0;
        let (Some(col), Some(row)) = (
            cut_index(&grid.x_cuts, center_x),
            cut_index(&grid.y_cuts, center_y),
        ) else {
            continue; // off-canvas: no area
        };
        let Some((piece_index, _)) = pieces.iter().enumerate().find(|(_, piece)| {
            piece.row <= row
                && row < piece.row + piece.row_span
                && piece.col <= col
                && col < piece.col + piece.col_span
        }) else {
            continue; // center over an omitted region: no area
        };
        // The area's hooks come from the hotspot's own behaviors; the
        // anchor name is the host image (the swap runtime addresses
        // images, and an area has none of its own).
        let host = naming::stem_of(&pieces[piece_index].file);
        let (over, out, click, _) = hooks(host, object.slice);
        anchors.push(Anchor {
            piece: piece_index,
            hotspot: index,
            over,
            out,
            click,
        });
    }
    anchors
}

/// Writes the HTML document. `pieces` must be the rasterizer's output
/// (piece order = file order).
pub(crate) fn write_html(
    document: &doc::Document,
    options: &WebExportOptions,
    objects: &[WebObject<'_>],
    grid: &Grid,
    pieces: &[Piece],
) -> String {
    let Some(page) = document.pages.get(options.page) else {
        return String::new();
    };
    let anchors = anchor_hotspots(objects, grid, pieces);
    let width = grid.x_cuts.last().copied().unwrap_or(0);

    // Behavior pass: runtime need and preload list, pieces first then
    // anchored hotspots, in emission order.
    let mut preloads: Vec<String> = Vec::new();
    let mut needs_runtime = false;
    let note = |hooks: (Option<String>, Option<String>, Option<String>, Vec<String>),
                preloads: &mut Vec<String>,
                needs: &mut bool| {
        let (over, out, click, preload) = hooks;
        *needs |= over.is_some() || out.is_some() || click.is_some();
        for file in preload {
            if !preloads.contains(&file) {
                preloads.push(file);
            }
        }
    };
    for piece in pieces {
        if let Some(index) = piece.object {
            let name = naming::stem_of(&piece.file);
            note(
                hooks(name, objects[index].slice),
                &mut preloads,
                &mut needs_runtime,
            );
        }
    }
    for anchor in &anchors {
        let hotspot = objects[anchor.hotspot].slice;
        note(hooks("area", hotspot), &mut preloads, &mut needs_runtime);
    }

    let mut html = String::with_capacity(4096);
    html.push_str("<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\" \"http://www.w3.org/TR/html4/loose.dtd\">\n");
    html.push_str("<html>\n<head>\n");
    html.push_str(&format!("<title>{}</title>\n", escape(&options.title)));
    html.push_str("<meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\">\n");
    html.push_str("<style type=\"text/css\">td img {display: block;}</style>\n");
    if needs_runtime {
        html.push_str("<script language=\"JavaScript\">\n<!--\n");
        html.push_str(MM_RUNTIME);
        html.push_str("//-->\n</script>\n");
    }
    html.push_str("</head>\n");
    let onload = if preloads.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = preloads.iter().map(|file| format!("'{file}'")).collect();
        format!(" onload=\"MM_preloadImages({})\"", list.join(","))
    };
    html.push_str(&format!(
        "<body bgcolor=\"{}\"{onload}>\n",
        bg_hex(page.info.canvas.background_raw)
    ));

    html.push_str("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\" ");
    match options.table_pct_width {
        Some(pct) => html.push_str(&format!("width=\"{pct}%\">\n")),
        None => html.push_str(&format!("width=\"{width}\">\n")),
    }

    // Shim row: one cell per column, then the 1px corner.
    if options.shim {
        html.push_str("  <tr>\n");
        for col in 0..grid.columns() {
            let (_, _, w, _) = grid.cell_rect(0, col);
            html.push_str(&format!(
                "   <td><img src=\"shim.gif\" width=\"{w}\" height=\"1\" alt=\"\"></td>\n"
            ));
        }
        html.push_str(
            "   <td><img src=\"shim.gif\" width=\"1\" height=\"1\" alt=\"\"></td>\n  </tr>\n",
        );
    }

    // Body rows: pieces at their top row; gaps between pieces are
    // cells no piece covers (skipped-undefined semantics).
    for row in 0..grid.rows() {
        let row_pieces: Vec<(usize, &Piece)> = pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.row == row)
            .collect();
        if row_pieces.is_empty() {
            continue;
        }
        html.push_str("  <tr>\n");
        let mut next_col = 0;
        for (piece_index, piece) in row_pieces {
            if piece.col < next_col {
                continue; // spanned by an earlier piece
            }
            next_col = piece.col + piece.col_span;
            write_piece(
                &mut html,
                options,
                objects,
                grid,
                piece_index,
                piece,
                &anchors,
            );
        }
        if options.shim {
            let (_, _, _, h) = grid.cell_rect(row, 0);
            html.push_str(&format!(
                "   <td><img src=\"shim.gif\" width=\"1\" height=\"{h}\" alt=\"\"></td>\n"
            ));
        }
        html.push_str("  </tr>\n");
    }
    html.push_str("</table>\n");

    // Image maps: one per host piece, first-seen piece order.
    let mut groups: Vec<(usize, Vec<&Anchor>)> = Vec::new();
    for anchor in &anchors {
        match groups.iter_mut().find(|(piece, _)| *piece == anchor.piece) {
            Some((_, members)) => members.push(anchor),
            None => groups.push((anchor.piece, vec![anchor])),
        }
    }
    for (piece_index, group) in &groups {
        let piece = &pieces[*piece_index];
        let host = naming::stem_of(&piece.file);
        html.push_str(&format!("<map name=\"m_{host}\">\n"));
        let (x, y, _, _) = grid.cell_rect(piece.row, piece.col);
        for anchor in group {
            let hotspot = objects[anchor.hotspot].slice;
            let (shape, coords) = area_geometry(hotspot, x, y);
            let href = hotspot
                .link
                .as_deref()
                .map(|link| format!(" href=\"{}\"", escape(link)))
                .unwrap_or_else(|| " href=\"javascript:;\"".to_owned());
            let alt = hotspot.alt.as_deref().map(escape).unwrap_or_default();
            html.push_str(&format!(
                "<area shape=\"{shape}\" coords=\"{coords}\"{href} title=\"{alt}\" alt=\"{alt}\""
            ));
            if let Some(code) = &anchor.out {
                html.push_str(&format!(" onMouseOut=\"{code}\""));
            }
            if let Some(code) = &anchor.over {
                html.push_str(&format!(" onMouseOver=\"{code}\""));
            }
            if let Some(code) = &anchor.click {
                html.push_str(&format!(" onClick=\"{code}\""));
            }
            html.push_str(">\n");
        }
        html.push_str("</map>\n");
    }

    html.push_str("</body>\n</html>\n");
    html
}

/// Emits one `<td>`: spans, anchor with hooks, the `<img>` (with
/// `usemap` when hotspots anchored here). `piece_index` is the
/// piece's position in the rasterizer's output.
fn write_piece(
    html: &mut String,
    options: &WebExportOptions,
    objects: &[WebObject<'_>],
    grid: &Grid,
    piece_index: usize,
    piece: &Piece,
    anchors: &[Anchor],
) {
    let (_, _, w, h) = grid.cell_rect(piece.row, piece.col);
    let slice = piece.object.map(|index| objects[index].slice);
    let mut td = "   <td".to_owned();
    if piece.row_span > 1 {
        td.push_str(&format!(" rowspan=\"{}\"", piece.row_span));
    }
    if piece.col_span > 1 {
        td.push_str(&format!(" colspan=\"{}\"", piece.col_span));
    }
    td.push_str(">\n");
    *html += &td;

    let name = naming::stem_of(&piece.file);
    let mut anchor_open = false;
    if let Some(slice) = slice {
        let (over, out, click, _) = hooks(name, slice);
        if over.is_some() || out.is_some() || click.is_some() || slice.link.is_some() {
            let href = slice
                .link
                .as_deref()
                .map(|link| format!(" href=\"{}\"", escape(link)))
                .unwrap_or_else(|| " href=\"javascript:;\"".to_owned());
            html.push_str(&format!("    <a{href}"));
            if let Some(code) = &out {
                html.push_str(&format!(" onMouseOut=\"{code}\""));
            }
            if let Some(code) = &over {
                html.push_str(&format!(" onMouseOver=\"{code}\""));
            }
            if let Some(code) = &click {
                html.push_str(&format!(" onClick=\"{code}\""));
            }
            html.push('>');
            anchor_open = true;
        }
    }
    let src = format!("{}{}", options.images_dir, piece.file);
    let alt = slice
        .and_then(|slice| slice.alt.as_deref())
        .map(escape)
        .unwrap_or_default();
    let usemap = if anchors.iter().any(|anchor| anchor.piece == piece_index) {
        format!(" usemap=\"#m_{name}\"")
    } else {
        String::new()
    };
    html.push_str(&format!(
        "    <img name=\"{name}\" src=\"{src}\" width=\"{w}\" height=\"{h}\"{usemap} alt=\"{alt}\">\n"
    ));
    if anchor_open {
        html.push_str("    </a>\n");
    }
    html.push_str("   </td>\n");
}
