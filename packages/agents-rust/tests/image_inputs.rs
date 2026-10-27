use frame_agent_connectors::{Prompt, PromptImage, Provider};
use serde_json::json;

#[test]
fn image_inputs_preserve_visual_bytes_or_owned_paths_for_each_provider() {
    let prompt = Prompt {
        text: "Draw".into(),
        turn_id: "turn".into(),
        images: vec![PromptImage {
            name: "ref.png".into(),
            path: "/owned/ref.png".into(),
            data: "cG5n".into(),
        }],
    };
    assert_eq!(
        prompt.parts(Provider::Codex)[1],
        json!({"type":"localImage","path":"/owned/ref.png"})
    );
    assert_eq!(
        prompt.parts(Provider::Gemini)[1],
        json!({"type":"image","data":"cG5n","mimeType":"image/png"})
    );
    assert_eq!(
        prompt.parts(Provider::Claude)[1]["source"],
        json!({"type":"base64","media_type":"image/png","data":"cG5n"})
    );
    assert_eq!(
        prompt.parts(Provider::Opencode)[1]["url"],
        "data:image/png;base64,cG5n"
    );
    let attachment: github_copilot_sdk::Attachment =
        serde_json::from_value(prompt.parts(Provider::Copilot)[1].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(attachment).unwrap()["path"],
        "/owned/ref.png"
    );
}
