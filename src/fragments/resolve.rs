use crate::error::DevshellError;
use crate::fragments::embedded;
use crate::fs;

pub fn resolve_fragment(reference: &str) -> Result<String, DevshellError> {
    if reference.starts_with('@') {
        resolve_embedded_fragment(&reference[1..])
    } else {
        resolve_disk_fragment(reference)
    }
}

pub fn resolve_fragments(references: &[String]) -> Result<Vec<String>, DevshellError> {
    let mut fragments = Vec::new();

    for reference in references {
        let fragment = resolve_fragment(reference)?;
        fragments.push(fragment);
    }

    Ok(fragments)
}

fn resolve_embedded_fragment(path: &str) -> Result<String, DevshellError> {
    let embedded = embedded::get_embedded_fragments();

    match embedded.get(path) {
        Some(content) => Ok((*content).to_string()),
        None => Err(DevshellError::FragmentNotFound(format!(
            "Embedded fragment '{}' not found",
            path
        ))),
    }
}

fn resolve_disk_fragment(path: &str) -> Result<String, DevshellError> {
    let fragment_path = fs::get_fragments_dir().join(format!("{}.docker", path));

    if !fragment_path.exists() {
        return Err(DevshellError::FragmentNotFound(format!(
            "Disk fragment '{}' not found at {}",
            path,
            fragment_path.display()
        )));
    }

    Ok(std::fs::read_to_string(fragment_path)?)
}

pub fn list_embedded_fragments() -> Vec<&'static str> {
    let embedded = embedded::get_embedded_fragments();
    embedded.keys().copied().collect()
}
