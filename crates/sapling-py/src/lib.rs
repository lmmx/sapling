use pyo3::prelude::*;
use sapling::*;
use std::collections::HashMap;

#[pymodule]
fn _sapling(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // m.add_class::<PyPatch>()?;
    // m.add_class::<PyPatchSet>()?;
    // m.add_class::<PySnippet>()?;
    // m.add_class::<PyBoundary>()?;
    // m.add_class::<PyTarget>()?;
    // m.add_function(wrap_pyfunction!(load_patches_from_json, m)?)?;
    // m.add_function(wrap_pyfunction!(save_patches_to_json, m)?)?;
    Ok(())
}
