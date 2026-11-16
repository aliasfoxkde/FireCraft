//! Build provenance (T27-56): an in-toto statement that binds every
//! published artifact's digest to the source revision, the workflow and
//! the runner.
//!
//! The statement is the release's own account of where its bytes came
//! from: the subject list carries the SHA-256 of each published asset,
//! and the predicate carries the source commit, the tag, the workflow
//! that ran and the runner it ran on. A consumer can check the whole
//! claim without trusting the release that published it — that is the
//! point of publishing it next to the artifacts.
//!
//! Two rules shape the code. The decision cores
//! ([`build_statement`] and [`verify_statement`]) take their inputs as
//! data, so a test never touches the environment, the git checkout or
//! the clock; only the edges ([`generate`] and [`verify`]) read them,
//! once. And the verification is fail-closed: a subject whose bytes
//! disagree with the recorded digest, a subject that is not one of the
//! release's published assets, an asset that no subject covers, or a
//! source revision that is not the one being built are all errors.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dist::{self, PROVENANCE_FILE_NAME};
use crate::util::workspace_root;

/// The in-toto statement type this module writes.
const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";

/// The predicate type: SLSA provenance v1.
const PREDICATE_TYPE: &str = "https://slsa.dev/provenance/v1";

/// The build type — the workflow that produces a release.
pub const BUILD_TYPE: &str =
    "https://github.com/aliasfoxkde/FireCraft/.github/workflows/release.yml";

/// The builder that produced the artifacts.
pub const BUILDER_ID: &str = "https://github.com/actions/runner/github-hosted";

/// The repository the artifacts were built from.
pub const REPOSITORY: &str = "https://github.com/aliasfoxkde/FireCraft";

/// One attested artifact: its published name and its SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Subject {
    /// The asset's published name — its basename, which is how a
    /// release names it.
    name: String,
    /// The asset's SHA-256 digest, lowercase hex.
    digest: String,
}

/// Builds the statement document.
///
/// The subjects are the artifacts; the predicate is the account of the
/// build that produced them. The same inputs always produce the same
/// document, so the statement can be regenerated and compared.
fn build_statement(subjects: &[Subject], ctx: &dist::ManifestContext) -> serde_json::Value {
    let subject_values: Vec<serde_json::Value> = subjects
        .iter()
        .map(|subject| {
            serde_json::json!({
                "name": subject.name,
                "digest": { "sha256": subject.digest },
            })
        })
        .collect();
    // The source URI names the tag when the build was cut from one and
    // the commit otherwise: a tag is what a consumer can fetch, and a
    // commit is what a build from a branch can prove.
    let source_ref = ctx.tag.clone().unwrap_or_else(|| ctx.source_sha.clone());
    let workflow_ref = ctx
        .tag
        .clone()
        .map(|tag| format!("refs/tags/{tag}"))
        .unwrap_or_else(|| ctx.source_sha.clone());
    serde_json::json!({
        "_type": STATEMENT_TYPE,
        "subject": subject_values,
        "predicateType": PREDICATE_TYPE,
        "predicate": {
            "buildDefinition": {
                "buildType": BUILD_TYPE,
                "externalParameters": {
                    "source": {
                        "uri": format!("git+{REPOSITORY}@{source_ref}"),
                        "digest": { "sha1": ctx.source_sha },
                    },
                    "workflow": {
                        "path": ".github/workflows/release.yml",
                        "ref": workflow_ref,
                        "repository": REPOSITORY,
                    },
                },
                "internalParameters": {
                    "runner": { "environment": "github-hosted" },
                },
            },
            "runDetails": {
                "builder": { "id": BUILDER_ID },
                "metadata": {
                    "invocationId": ctx.run_url,
                    "startedOn": ctx.built_at,
                    "finishedOn": ctx.built_at,
                },
            },
        },
    })
}

/// Renders the statement: pretty-printed JSON with a trailing newline.
fn render(value: &serde_json::Value) -> Result<String, String> {
    let mut text =
        serde_json::to_string_pretty(value).map_err(|e| format!("render the statement: {e}"))?;
    text.push('\n');
    Ok(text)
}

/// Indexes every file under `root` by its basename.
///
/// A release asset is named by its basename, so the basename is how a
/// downloaded artifact is identified. Two files with one basename are
/// an error rather than a coin toss: the statement would attest to
/// one of them.
fn index_by_basename(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut index: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if index.insert(name.clone(), path).is_some() {
                return Err(format!(
                    "two files share the name {name} under {} — a release asset is named by its basename",
                    root.display()
                ));
            }
        }
    }
    Ok(index)
}

/// Reads the statement's subject list.
///
/// A subject without a name, or without a SHA-256 digest, is a broken
/// statement — not a subject to skip.
fn read_subjects(statement: &serde_json::Value) -> Result<Vec<Subject>, String> {
    let subjects = statement
        .get("subject")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "the statement carries no subject list".to_owned())?;
    if subjects.is_empty() {
        return Err("the statement carries an empty subject list".to_owned());
    }
    let mut parsed = Vec::with_capacity(subjects.len());
    for (index, value) in subjects.iter().enumerate() {
        let name = value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| format!("subject {index} has no name"))?
            .to_owned();
        let digest = value
            .get("digest")
            .and_then(|digest| digest.get("sha256"))
            .and_then(serde_json::Value::as_str)
            .filter(|digest| digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| format!("subject {name} has no SHA-256 digest"))?
            .to_ascii_lowercase();
        parsed.push(Subject { name, digest });
    }
    Ok(parsed)
}

/// The source revision the statement claims, if it claims one.
fn read_source_sha(statement: &serde_json::Value) -> Result<String, String> {
    statement
        .get("predicate")
        .and_then(|predicate| predicate.get("buildDefinition"))
        .and_then(|definition| definition.get("externalParameters"))
        .and_then(|parameters| parameters.get("source"))
        .and_then(|source| source.get("digest"))
        .and_then(|digest| digest.get("sha1"))
        .and_then(serde_json::Value::as_str)
        .filter(|sha| sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_owned)
        .ok_or_else(|| "the statement records no source revision".to_owned())
}

/// Checks that the predicate carries the fields a consumer relies on.
///
/// A statement whose predicate is missing its build type, source or
/// builder says less than it appears to; the fields are the claim, so
/// their absence is an error.
fn check_predicate(statement: &serde_json::Value) -> Result<(), String> {
    if statement.get("_type").and_then(serde_json::Value::as_str) != Some(STATEMENT_TYPE) {
        return Err("the statement is not an in-toto v1 statement".to_owned());
    }
    if statement
        .get("predicateType")
        .and_then(serde_json::Value::as_str)
        != Some(PREDICATE_TYPE)
    {
        return Err("the statement carries an unknown predicate type".to_owned());
    }
    let predicate = statement
        .get("predicate")
        .ok_or_else(|| "the statement carries no predicate".to_owned())?;
    let build_type = predicate
        .get("buildDefinition")
        .and_then(|definition| definition.get("buildType"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "the predicate records no build type".to_owned())?;
    if build_type != BUILD_TYPE {
        return Err(format!(
            "the predicate names another build type: {build_type}"
        ));
    }
    let source_uri = predicate
        .get("buildDefinition")
        .and_then(|definition| definition.get("externalParameters"))
        .and_then(|parameters| parameters.get("source"))
        .and_then(|source| source.get("uri"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "the predicate records no source URI".to_owned())?;
    if !source_uri.starts_with("git+") {
        return Err(format!("the source URI is not a git URI: {source_uri}"));
    }
    let builder = predicate
        .get("runDetails")
        .and_then(|details| details.get("builder"))
        .and_then(|builder| builder.get("id"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "the predicate records no builder".to_owned())?;
    if builder != BUILDER_ID {
        return Err(format!("the predicate names another builder: {builder}"));
    }
    Ok(())
}

/// Verifies a statement against the files in `root`.
///
/// `expected_source_sha` is the revision the caller believes it is
/// building; when it is given, the statement must claim that revision.
/// `expected_subjects` is the set of published asset names the release
/// is expected to attest to; every one of them must be covered, and
/// nothing outside them may be.
fn verify_statement(
    root: &Path,
    statement: &serde_json::Value,
    expected_source_sha: Option<&str>,
    expected_subjects: Option<&[String]>,
) -> Result<(), String> {
    check_predicate(statement)?;
    let subjects = read_subjects(statement)?;
    let source_sha = read_source_sha(statement)?;
    if let Some(expected) = expected_source_sha
        && source_sha != expected
    {
        return Err(format!(
            "the statement was made from {source_sha}, this build is {expected}"
        ));
    }
    if let Some(expected) = expected_subjects {
        let covered: Vec<&str> = subjects
            .iter()
            .map(|subject| subject.name.as_str())
            .collect();
        for name in expected {
            if !covered.contains(&name.as_str()) {
                return Err(format!("no subject covers {name}"));
            }
        }
        for subject in &subjects {
            if !expected.iter().any(|name| name == &subject.name) {
                return Err(format!(
                    "the statement attests to {}, which is not a published asset",
                    subject.name
                ));
            }
        }
    }
    let index = index_by_basename(root)?;
    for subject in &subjects {
        let path = index.get(&subject.name).ok_or_else(|| {
            format!(
                "the statement attests to {}, which is not in {}",
                subject.name,
                root.display()
            )
        })?;
        let actual = dist::sha256_hex_file(path)?;
        if actual != subject.digest {
            return Err(format!(
                "digest mismatch for {}: the statement records {}, the file hashes to {actual}",
                subject.name, subject.digest
            ));
        }
    }
    Ok(())
}

/// The published asset names the statement must cover: every release
/// artifact except the statement itself.
fn expected_subject_names(version: &str) -> Vec<String> {
    dist::release_specs(version)
        .into_iter()
        .filter(|spec| spec.name != PROVENANCE_FILE_NAME)
        .map(|spec| spec.name)
        .collect()
}

/// Writes the provenance statement next to the artifacts (T27-56).
///
/// The build-time facts are read once, at the edge, and passed in as
/// data; the statement bytes are therefore a function of the artifact
/// set and those facts.
pub fn generate() -> Result<(), String> {
    let version = dist::workspace_version()?;
    let out = workspace_root().join("dist");
    let ctx = dist::manifest_context(&version)?;
    let specs = dist::release_specs(&version);
    let mut subjects = Vec::with_capacity(specs.len());
    for spec in &specs {
        if spec.name == PROVENANCE_FILE_NAME {
            // The statement does not attest to itself.
            continue;
        }
        let path = out.join(&spec.rel_path);
        if !path.is_file() {
            return Err(format!("release artifact missing: {}", spec.name));
        }
        let (digest, size) = dist::digest_and_size(&path)?;
        if size == 0 {
            return Err(format!("release artifact is empty: {}", spec.name));
        }
        subjects.push(Subject {
            name: spec.name.clone(),
            digest,
        });
    }
    let statement = build_statement(&subjects, &ctx);
    let text = render(&statement)?;
    let path = out.join(PROVENANCE_FILE_NAME);
    std::fs::write(&path, &text).map_err(|e| format!("write {}: {e}", path.display()))?;
    eprintln!(
        "[xtask] dist provenance: {} artifact(s) -> {}",
        subjects.len(),
        path.display()
    );
    Ok(())
}

/// Verifies `dist/provenance.intoto.json` against the files on disk.
///
/// Every attested artifact must exist with the digest the statement
/// records, every published artifact must be covered, and the source
/// revision the statement claims must be the revision this tree is
/// built from.
pub fn verify() -> Result<(), String> {
    let version = dist::workspace_version()?;
    let out = workspace_root().join("dist");
    let path = out.join(PROVENANCE_FILE_NAME);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let statement: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let ctx = dist::manifest_context(&version)?;
    let expected = expected_subject_names(&version);
    verify_statement(&out, &statement, Some(&ctx.source_sha), Some(&expected))?;
    eprintln!(
        "[xtask] dist verify-provenance: {} artifact(s) verified against {}",
        expected.len(),
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod provenance_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn context() -> dist::ManifestContext {
        dist::ManifestContext {
            version: "0.2.6".to_owned(),
            tag: Some("v0.2.6".to_owned()),
            source_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            run_url: Some("https://github.com/aliasfoxkde/FireCraft/actions/runs/1".to_owned()),
            run_id: Some("1".to_owned()),
            built_at: "2026-11-16T00:00:00Z".to_owned(),
        }
    }

    fn subject(name: &str, digest: &str) -> Subject {
        Subject {
            name: name.to_owned(),
            digest: digest.to_owned(),
        }
    }

    /// The SHA-256 of the bytes the tests write — a real digest of the
    /// real payload, not a placeholder.
    const PAYLOAD_DIGEST: &str = "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5";

    #[test]
    fn the_statement_binds_the_subjects_to_the_source_revision() {
        let statement = build_statement(&[subject("firecraft.tar.gz", PAYLOAD_DIGEST)], &context());
        assert_eq!(statement["_type"], STATEMENT_TYPE);
        assert_eq!(statement["predicateType"], PREDICATE_TYPE);
        assert_eq!(statement["subject"][0]["name"], "firecraft.tar.gz");
        assert_eq!(statement["subject"][0]["digest"]["sha256"], PAYLOAD_DIGEST);
        let source = &statement["predicate"]["buildDefinition"]["externalParameters"]["source"];
        assert_eq!(
            source["digest"]["sha1"],
            "0123456789abcdef0123456789abcdef01234567"
        );
        assert_eq!(
            source["uri"],
            "git+https://github.com/aliasfoxkde/FireCraft@v0.2.6"
        );
    }

    #[test]
    fn a_statement_without_a_tag_names_the_commit() {
        let mut ctx = context();
        ctx.tag = None;
        let statement = build_statement(&[subject("firecraft.tar.gz", PAYLOAD_DIGEST)], &ctx);
        let source = &statement["predicate"]["buildDefinition"]["externalParameters"]["source"];
        assert_eq!(
            source["uri"],
            "git+https://github.com/aliasfoxkde/FireCraft@0123456789abcdef0123456789abcdef01234567"
        );
    }

    #[test]
    fn a_verified_statement_passes_against_the_files_it_names() {
        let dir = tempdir("verified");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        let expected = vec!["firecraft.tar.gz".to_owned()];
        verify_statement(&dir, &statement, None, Some(&expected)).unwrap();
    }

    #[test]
    fn a_changed_artifact_fails_the_verification() {
        let dir = tempdir("changed");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        std::fs::write(&file, b"tampered").unwrap();
        let expected = vec!["firecraft.tar.gz".to_owned()];
        let error = verify_statement(&dir, &statement, None, Some(&expected)).unwrap_err();
        assert!(error.contains("digest mismatch"), "{error}");
    }

    #[test]
    fn an_artifact_no_subject_covers_fails_the_verification() {
        let dir = tempdir("uncovered");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        let expected = vec![
            "firecraft.tar.gz".to_owned(),
            "firecraft-cli.tar.gz".to_owned(),
        ];
        let error = verify_statement(&dir, &statement, None, Some(&expected)).unwrap_err();
        assert!(error.contains("no subject covers"), "{error}");
    }

    #[test]
    fn a_statement_for_another_revision_fails_the_verification() {
        let dir = tempdir("revision");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        let error = verify_statement(
            &dir,
            &statement,
            Some("ffffffffffffffffffffffffffffffffffffffff"),
            None,
        )
        .unwrap_err();
        assert!(error.contains("this build is"), "{error}");
    }

    #[test]
    fn a_statement_without_a_source_revision_is_refused() {
        let dir = tempdir("nosource");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let mut statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        statement["predicate"]["buildDefinition"]["externalParameters"]["source"]["digest"] =
            serde_json::json!({});
        let error = verify_statement(&dir, &statement, None, None).unwrap_err();
        assert!(error.contains("no source revision"), "{error}");
    }

    #[test]
    fn a_statement_for_another_builder_is_refused() {
        let dir = tempdir("builder");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let digest = dist::sha256_hex_file(&file).unwrap();
        let mut statement = build_statement(&[subject("firecraft.tar.gz", &digest)], &context());
        statement["predicate"]["runDetails"]["builder"]["id"] =
            serde_json::json!("https://example");
        let error = verify_statement(&dir, &statement, None, None).unwrap_err();
        assert!(error.contains("another builder"), "{error}");
    }

    #[test]
    fn a_subject_with_no_digest_is_refused() {
        let dir = tempdir("nodigest");
        let file = dir.join("firecraft.tar.gz");
        std::fs::write(&file, b"payload").unwrap();
        let mut statement =
            build_statement(&[subject("firecraft.tar.gz", PAYLOAD_DIGEST)], &context());
        statement["subject"][0]["digest"] = serde_json::json!({});
        let error = verify_statement(&dir, &statement, None, None).unwrap_err();
        assert!(error.contains("no SHA-256 digest"), "{error}");
    }

    /// A scratch directory under the system temp dir.
    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("firecraft-provenance-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
