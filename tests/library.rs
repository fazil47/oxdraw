use anyhow::Result;
use oxdraw::{
    AddEdgeInput, AddNodeInput, Diagram, EdgeArrowDirection, EdgeKind, EditorCore, RenameLabelInput,
};

#[test]
fn diagram_parse_and_render_svg() -> Result<()> {
    let definition = r#"
        graph TD
            A[Start] -->|process| B[End]
    "#;

    let diagram = Diagram::parse(definition)?;
    let svg = diagram.render_svg("white", None)?;

    assert!(
        svg.contains("<svg"),
        "rendered svg should contain root element"
    );
    assert!(svg.contains("Start"), "node labels should appear in output");

    Ok(())
}

#[test]
fn diagram_render_png_has_png_header() -> Result<()> {
    let definition = r#"
        graph LR
            Start --> Finish
    "#;

    let diagram = Diagram::parse(definition)?;
    let png = diagram.render_png("white", None, 2.0)?;

    const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    assert!(
        png.starts_with(PNG_MAGIC),
        "rendered png should start with PNG header"
    );

    Ok(())
}

#[test]
fn diagram_parses_image_comments() -> Result<()> {
    let definition = include_str!("input/image_node.mmd");
    let diagram = Diagram::parse(definition)?;

    let (node_id, node) = diagram
        .nodes
        .iter()
        .find(|(_, node)| node.image.is_some())
        .expect("expected an image node to be present");
    let image = node
        .image
        .as_ref()
        .expect("expected node image to be parsed");

    assert_eq!(image.mime_type, "image/png");
    assert!(!image.data.is_empty(), "image payload should not be empty");

    let svg = diagram.render_svg("white", None)?;
    assert!(
        svg.contains(&format!("clip-path=\"url(#oxdraw-node-clip-{})\"", node_id)),
        "rendered svg should reference the node clip path"
    );
    assert!(
        svg.contains("data:image/png;base64,"),
        "rendered svg should contain a data URI for the embedded image"
    );

    Ok(())
}

#[test]
fn diagram_adds_node_to_flowchart_definition() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start]\n")?;

    let changed = diagram.add_node(AddNodeInput {
        id: "B".to_string(),
        label: Some("Next".to_string()),
        ..Default::default()
    })?;

    assert!(changed);
    assert!(diagram.nodes.contains_key("B"));
    assert!(diagram.to_definition().contains("B[Next]"));
    Ok(())
}

#[test]
fn diagram_rejects_duplicate_node_id() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start]\n")?;

    let changed = diagram.add_node(AddNodeInput {
        id: "A".to_string(),
        label: Some("Duplicate".to_string()),
        ..Default::default()
    })?;

    assert!(!changed);
    assert_eq!(diagram.nodes["A"].label, "Start");
    Ok(())
}

#[test]
fn diagram_adds_edge_between_existing_nodes() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start]\n    B[End]\n")?;

    let changed = diagram.add_edge(AddEdgeInput {
        from: "A".to_string(),
        to: "B".to_string(),
        label: Some("go".to_string()),
        kind: EdgeKind::Solid,
        arrow: EdgeArrowDirection::Forward,
    })?;

    assert!(changed);
    let definition = diagram.to_definition();
    assert!(definition.contains("A -->|go| B"));
    let reparsed = Diagram::parse(&definition)?;
    assert_eq!(reparsed.edges.len(), 1);
    Ok(())
}

#[test]
fn diagram_rejects_edge_with_missing_endpoint() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start]\n")?;

    let err = diagram
        .add_edge(AddEdgeInput {
            from: "A".to_string(),
            to: "Missing".to_string(),
            ..Default::default()
        })
        .expect_err("missing endpoint should fail");

    assert!(err.to_string().contains("target node 'Missing' not found"));
    Ok(())
}

#[test]
fn diagram_renames_node_label() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start]\n")?;

    let changed = diagram.rename_node("A", Some("Renamed"))?;

    assert!(changed);
    assert!(diagram.to_definition().contains("A[Renamed]"));
    Ok(())
}

#[test]
fn diagram_renames_edge_label_and_can_remove_it() -> Result<()> {
    let mut diagram = Diagram::parse("graph TD\n    A[Start] -->|old| B[End]\n")?;

    assert!(diagram.rename_edge("A --> B", Some("new"))?);
    assert!(diagram.to_definition().contains("A -->|new| B"));

    assert!(diagram.rename_edge("A --> B", None)?);
    assert!(diagram.to_definition().contains("A --> B"));
    Ok(())
}

#[test]
fn editor_core_adds_then_deletes_node_and_edge() -> Result<()> {
    let mut core = EditorCore::from_source("graph TD\n    A[Start]\n", "white")?;

    assert!(core.add_node(AddNodeInput {
        id: "B".to_string(),
        label: Some("End".to_string()),
        ..Default::default()
    })?);
    assert!(core.add_edge(AddEdgeInput {
        from: "A".to_string(),
        to: "B".to_string(),
        ..Default::default()
    })?);
    assert!(core.source()?.contains("A --> B"));

    assert!(core.delete_edge("A --> B")?);
    assert!(core.delete_node("B")?);
    assert!(!core.source()?.contains("B[End]"));

    Ok(())
}

#[test]
fn editor_core_renames_labels() -> Result<()> {
    let mut core = EditorCore::from_source("graph TD\n    A[Start] -->|old| B[End]\n", "white")?;

    assert!(core.rename_node(
        "A",
        RenameLabelInput {
            label: Some("Begin".to_string()),
        },
    )?);
    assert!(core.rename_edge(
        "A --> B",
        RenameLabelInput {
            label: Some("next".to_string()),
        },
    )?);

    let source = core.source()?;
    assert!(source.contains("A[Begin]"));
    assert!(source.contains("A -->|next| B"));
    Ok(())
}
