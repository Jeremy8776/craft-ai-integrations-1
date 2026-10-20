#[path = "../../ai-openai/tests/support/mod.rs"]
mod support;
use craft_ai_comfyui::{Sam1Session, Sam2Session};
use serde_json::{Value, json};
use support::{Reply, Server};

fn png() -> Vec<u8> {
    let mut data = vec![];
    let mut encoder = png::Encoder::new(&mut data, 64, 64);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&vec![255; 4096])
        .unwrap();
    data
}

fn server() -> Server {
    let mask = png();
    Server::start(move |r| {
        match r.path.as_str() {
        "/system_stats" => Reply::Json(200, json!({"system":{"comfyui_version":"0.22.3"}}).to_string()),
        "/object_info" => Reply::Json(200, json!({"LoadImage":{},"Florence2ModelLoader":{"input":{"required":{"model":[["Florence-2-large"],{}]}}},"Florence2Run":{},"Florence2toCoordinates":{},"DownloadAndLoadSAM2Model":{},"Sam2Segmentation":{},"SAMModelLoader (segment anything)":{},"GroundingDinoModelLoader (segment anything)":{},"GroundingDinoSAMSegment (segment anything)":{},"MaskToImage":{},"SaveImage":{}}).to_string()),
        "/upload/image" => Reply::Json(200,json!({"name":"source.png","type":"input","subfolder":""}).to_string()),
        "/prompt" => Reply::Json(200,json!({"prompt_id":"sam-job"}).to_string()),
        "/history/sam-job" => Reply::Json(200,json!({"sam-job":{"status":{"completed":true},"outputs":{"8":{"images":[{"filename":"mask.png","type":"output","subfolder":""}]}}}}).to_string()),
        "/view?filename=mask.png&type=output" => Reply::Raw([format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",mask.len()).into_bytes(),mask.clone()].concat()),
        _ => Reply::Json(404,"{}".into()),
    }
    })
}

#[test]
fn sam2_uses_florence_boxes_and_the_selected_sam2_model() {
    let server = server();
    let mut session = Sam2Session::new(
        server.url.trim_end_matches("/v1"),
        "sam2.1_hiera_large.safetensors",
        "cuda",
        "fp16",
    )
    .unwrap();
    session.prepare(&png()).unwrap();
    assert_eq!(session.ground("person").unwrap(), png());
    let calls = server.recorded();
    let graph: Value = calls
        .iter()
        .find(|r| r.path == "/prompt")
        .unwrap()
        .body_json()["prompt"]
        .clone();
    assert_eq!(graph["2"]["inputs"]["model"], "Florence-2-large");
    assert_eq!(graph["3"]["inputs"]["task"], "caption_to_phrase_grounding");
    assert_eq!(graph["5"]["class_type"], "DownloadAndLoadSAM2Model");
    assert_eq!(
        graph["5"]["inputs"]["model"],
        "sam2.1_hiera_large.safetensors"
    );
    assert_eq!(graph["5"]["inputs"]["device"], "cuda");
    assert_eq!(graph["5"]["inputs"]["precision"], "fp16");
    assert_eq!(graph["6"]["inputs"]["bboxes"], json!(["4", 1]));
    assert_eq!(graph["8"]["class_type"], "SaveImage");
}

#[test]
fn sam1_uses_grounding_dino_and_sam_models_without_falling_back() {
    let server = server();
    let mut session = Sam1Session::new(
        server.url.trim_end_matches("/v1"),
        "sam_vit_b (375MB)",
        "GroundingDINO_SwinT_OGC (694MB)",
    )
    .unwrap();
    session.prepare(&png()).unwrap();
    assert_eq!(session.ground("person").unwrap(), png());
    let calls = server.recorded();
    let graph: Value = calls
        .iter()
        .find(|r| r.path == "/prompt")
        .unwrap()
        .body_json()["prompt"]
        .clone();
    assert_eq!(
        graph["2"]["class_type"],
        "SAMModelLoader (segment anything)"
    );
    assert_eq!(graph["2"]["inputs"]["model_name"], "sam_vit_b (375MB)");
    assert_eq!(
        graph["3"]["class_type"],
        "GroundingDinoModelLoader (segment anything)"
    );
    assert_eq!(
        graph["3"]["inputs"]["model_name"],
        "GroundingDINO_SwinT_OGC (694MB)"
    );
    assert_eq!(
        graph["4"]["class_type"],
        "GroundingDinoSAMSegment (segment anything)"
    );
    assert_eq!(graph["4"]["inputs"]["prompt"], "person");
    assert_eq!(graph["5"]["class_type"], "MaskToImage");
}

#[test]
#[ignore = "requires the existing local ComfyUI, SAM 2.1 and Florence-2 files"]
fn installed_sam2_florence_grounding_returns_a_checked_mask() {
    let url = std::env::var("CRAFT_TEST_COMFYUI_URL").expect("local ComfyUI URL");
    let image =
        std::fs::read(std::env::var("CRAFT_TEST_SAM12_IMAGE").expect("test PNG path")).unwrap();
    let source = png::Decoder::new(image.as_slice()).read_info().unwrap();
    let mut session =
        Sam2Session::new(&url, "sam2.1_hiera_large.safetensors", "cuda", "fp16").unwrap();
    session.verify_server().unwrap();
    session.prepare(&image).unwrap();
    let mask = session.ground("red square").unwrap();
    let mut decoder = png::Decoder::new(mask.as_slice()).read_info().unwrap();
    assert_eq!(
        (decoder.info().width, decoder.info().height),
        (source.info().width, source.info().height)
    );
    let mut pixels = vec![0; decoder.output_buffer_size()];
    let frame = decoder.next_frame(&mut pixels).unwrap();
    assert!(pixels[..frame.buffer_size()].iter().any(|pixel| *pixel > 0));
    assert!(pixels[..frame.buffer_size()].contains(&0));
}

fn red_square_png() -> Vec<u8> {
    let mut pixels = vec![0u8; 256 * 256 * 3];
    for y in 64..192 {
        for x in 64..192 {
            let i = (y * 256 + x) * 3;
            pixels[i] = 255;
        }
    }
    let mut image = Vec::new();
    let mut encoder = png::Encoder::new(&mut image, 256, 256);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    image
}

#[test]
#[ignore = "requires the existing local ComfyUI, SAM 1, GroundingDINO and BERT files"]
fn installed_sam1_grounding_dino_returns_a_checked_mask() {
    let url = std::env::var("CRAFT_TEST_COMFYUI_URL").expect("local ComfyUI URL");
    let image = red_square_png();
    let source = png::Decoder::new(image.as_slice()).read_info().unwrap();
    let mut session =
        Sam1Session::new(&url, "sam_vit_b (375MB)", "GroundingDINO_SwinT_OGC (694MB)").unwrap();
    session.verify_server().unwrap();
    session.prepare(&image).unwrap();
    let mask = session.ground("red square").unwrap();
    let mut decoder = png::Decoder::new(mask.as_slice()).read_info().unwrap();
    assert_eq!(
        (decoder.info().width, decoder.info().height),
        (source.info().width, source.info().height)
    );
    let mut pixels = vec![0; decoder.output_buffer_size()];
    let frame = decoder.next_frame(&mut pixels).unwrap();
    let mask = &pixels[..frame.buffer_size()];
    assert!(mask.iter().any(|pixel| *pixel > 0));
    assert!(mask.contains(&0));
}
