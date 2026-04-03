use crate::prelude::*;

/// Stores named groups of mesh elements for easy retrieval.
///
/// Tags let you label selections during construction and retrieve them later
/// by name, avoiding the need to track element IDs through long procedural chains.
///
/// ```
/// use glam::vec3;
/// use smesh::prelude::*;
///
/// let mesh = &mut SMesh::new();
/// let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
/// let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
/// let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
/// let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
/// let f = mesh.make_quad(v0, v1, v2, v3).unwrap();
///
/// mesh.tag(f, "base");
/// let base = mesh.get_tag("base").unwrap();
/// ```
impl SMesh {
    /// Tag a selection with a name. Overwrites any existing tag with the same name.
    pub fn tag<S: Into<MeshSelection>>(&mut self, selection: S, name: &str) {
        self.tags.insert(name.to_string(), selection.into());
    }

    /// Append elements to an existing tag, or create it if it doesn't exist.
    pub fn tag_append<S: Into<MeshSelection>>(&mut self, selection: S, name: &str) {
        let new_selection = selection.into();
        if let Some(existing) = self.tags.get_mut(name) {
            existing.merge(&new_selection);
        } else {
            self.tags.insert(name.to_string(), new_selection);
        }
    }

    /// Retrieve a tagged selection by name.
    pub fn get_tag(&self, name: &str) -> Option<&MeshSelection> {
        self.tags.get(name)
    }

    /// Retrieve a tagged selection by name, cloned for use in mutation operations.
    pub fn take_tag(&self, name: &str) -> Option<MeshSelection> {
        self.tags.get(name).cloned()
    }

    /// Remove a tag by name, returning the selection if it existed.
    pub fn remove_tag(&mut self, name: &str) -> Option<MeshSelection> {
        self.tags.remove(name)
    }

    /// List all tag names.
    pub fn tag_names(&self) -> Vec<&str> {
        self.tags.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a tag exists.
    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.contains_key(name)
    }

    /// Remove all tags.
    pub fn clear_tags(&mut self) {
        self.tags.clear();
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn tag_and_retrieve_face() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        mesh.tag(f, "floor");
        assert!(mesh.has_tag("floor"));
        assert!(!mesh.has_tag("roof"));

        let sel = mesh.get_tag("floor").unwrap();
        let faces = sel.resolve_to_faces(mesh)?;
        assert!(faces.contains(&f));

        Ok(())
    }

    #[test]
    fn tag_vertices() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        mesh.make_triangle(v0, v1, v2)?;

        mesh.tag(vec![v0, v1], "bottom_edge");
        let sel = mesh.get_tag("bottom_edge").unwrap();
        let verts = sel.resolve_to_vertices(mesh)?;
        assert!(verts.contains(&v0));
        assert!(verts.contains(&v1));
        assert!(!verts.contains(&v2));

        Ok(())
    }

    #[test]
    fn tag_append() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let f0 = mesh.make_triangle(v0, v1, v2)?;
        let f1 = mesh.make_triangle(v1, v3, v2)?;

        mesh.tag(f0, "group");
        mesh.tag_append(f1, "group");

        let sel = mesh.take_tag("group").unwrap();
        let faces = sel.resolve_to_faces(mesh)?;
        assert!(faces.contains(&f0));
        assert!(faces.contains(&f1));

        Ok(())
    }

    #[test]
    fn remove_tag() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let f = mesh.make_triangle(v0, v1, v2)?;

        mesh.tag(f, "temp");
        assert!(mesh.has_tag("temp"));
        mesh.remove_tag("temp");
        assert!(!mesh.has_tag("temp"));

        Ok(())
    }

    #[test]
    fn tag_names() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let f = mesh.make_triangle(v0, v1, v2)?;

        mesh.tag(f, "a");
        mesh.tag(vec![v0], "b");
        let names = mesh.tag_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));

        Ok(())
    }

    #[test]
    fn take_tag_for_mutation() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        mesh.tag(f, "floor");
        let sel = mesh.take_tag("floor").unwrap();
        // Should be usable with translate (needs owned MeshSelection)
        mesh.translate(sel, glam::Vec3::Y)?;

        let new_pos = v0.position(mesh)?;
        assert!((new_pos.y - 1.0).abs() < 1e-6);

        Ok(())
    }
}
