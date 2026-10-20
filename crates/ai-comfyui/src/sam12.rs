//! SAM 1 and SAM 2 text-grounded local selection workflows over ComfyUI.
use crate::http::Http;
use craft_ai_core::endpoint::Endpoint;
use craft_ai_core::{AiError, ErrorKind};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const IMAGE_LIMIT: usize = 16 * 1024 * 1024;
const PIXEL_LIMIT: u64 = 16_777_216;
const SAM2_NODES: &[&str] = &[
    "LoadImage",
    "Florence2ModelLoader",
    "Florence2Run",
    "Florence2toCoordinates",
    "DownloadAndLoadSAM2Model",
    "Sam2Segmentation",
    "MaskToImage",
    "SaveImage",
];
const SAM1_NODES: &[&str] = &[
    "LoadImage",
    "SAMModelLoader (segment anything)",
    "GroundingDinoModelLoader (segment anything)",
    "GroundingDinoSAMSegment (segment anything)",
    "MaskToImage",
    "SaveImage",
];

struct Prepared {
    filename: String,
    width: u32,
    height: u32,
}
struct Session {
    http: Http,
    prepared: Option<Prepared>,
}
impl Session {
    fn new(base: &str) -> Result<Self, AiError> {
        let endpoint =
            Endpoint::parse(base).map_err(|_| AiError::invalid("invalid ComfyUI address"))?;
        if !endpoint.is_loopback() || base != endpoint.fingerprint() {
            return Err(AiError::invalid(
                "selection needs a loopback ComfyUI address without a path",
            ));
        }
        Ok(Self {
            http: Http::new(
                base.to_owned(),
                Duration::from_secs(3),
                Duration::from_secs(15),
            ),
            prepared: None,
        })
    }
    fn verify_server(&self) -> Result<(), AiError> {
        let stats = self.http.get_json("/system_stats")?;
        if stats
            .pointer("/system/comfyui_version")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(AiError::unsupported(
                "The local port is not serving ComfyUI",
            ));
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        image: &[u8],
        required: &[&str],
        prefix: &str,
        needs_detector: bool,
    ) -> Result<(), AiError> {
        self.prepared = None;
        if image.len() < 8 || image.len() > IMAGE_LIMIT || !image.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err(AiError::invalid("selection needs a PNG image under 16 MiB"));
        }
        let reader = png::Decoder::new(image)
            .read_info()
            .map_err(|_| AiError::invalid("selection needs a valid PNG image"))?;
        let (width, height) = (reader.info().width, reader.info().height);
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > PIXEL_LIMIT
        {
            return Err(AiError::invalid("selection image dimensions are invalid"));
        }
        self.verify_server()?;
        let nodes = self.http.get_object_info()?;
        for class in required {
            if nodes.get(*class).is_none() {
                return Err(AiError::unsupported(format!(
                    "ComfyUI is missing the {class} node"
                )));
            }
        }
        if needs_detector {
            crate::detector::require_detector(&nodes)?;
        }
        let filename = self.http.upload_image(prefix, "image/png", image)?;
        self.prepared = Some(Prepared {
            filename,
            width,
            height,
        });
        Ok(())
    }
    fn run(&self, graph: Value, text: &str) -> Result<Vec<u8>, AiError> {
        let p = self
            .prepared
            .as_ref()
            .ok_or_else(|| AiError::invalid("selection image not prepared"))?;
        let text = text.trim();
        if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
            return Err(AiError::invalid("selection needs a short subject name"));
        }
        let response = self.http.post("/prompt", &json!({"prompt":graph}))?;
        let id = response
            .get("prompt_id")
            .and_then(Value::as_str)
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
            })
            .ok_or_else(|| {
                AiError::new(
                    ErrorKind::Provider,
                    "ComfyUI did not return a valid prompt id",
                )
            })?;
        let deadline = Instant::now() + Duration::from_secs(240);
        loop {
            let history = self.http.get_json(&format!("/history/{id}"))?;
            if let Some(record) = history.get(id) {
                if record.pointer("/status/status_str").and_then(Value::as_str) == Some("error") {
                    return Err(AiError::new(
                        ErrorKind::Provider,
                        "selection workflow failed in ComfyUI",
                    ));
                }
                if record.pointer("/status/completed").and_then(Value::as_bool) == Some(true) {
                    let output = record.pointer("/outputs/8").ok_or_else(|| {
                        AiError::new(ErrorKind::Provider, "selection returned no mask")
                    })?;
                    let masks = crate::sam3_masks::fetch(&self.http, output, p.width, p.height)?;
                    if !masks.has_pixels()? {
                        return Err(AiError::new(
                            ErrorKind::Provider,
                            format!("Nothing matched {text}"),
                        ));
                    }
                    return masks.union_png();
                }
            }
            if Instant::now() >= deadline {
                return Err(AiError::new(
                    ErrorKind::Network,
                    "selection timed out in ComfyUI",
                ));
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}

/// Florence-2 grounding followed by the explicitly selected SAM 2 checkpoint. Calls block; use a worker.
pub struct Sam2Session {
    session: Session,
    model: String,
    device: String,
    precision: String,
}
impl Sam2Session {
    pub fn new(base: &str, model: &str, device: &str, precision: &str) -> Result<Self, AiError> {
        let models = [
            "sam2_hiera_base_plus.safetensors",
            "sam2_hiera_large.safetensors",
            "sam2_hiera_small.safetensors",
            "sam2_hiera_tiny.safetensors",
            "sam2.1_hiera_base_plus.safetensors",
            "sam2.1_hiera_large.safetensors",
            "sam2.1_hiera_small.safetensors",
            "sam2.1_hiera_tiny.safetensors",
        ];
        if !models.contains(&model)
            || !["cuda", "cpu", "mps"].contains(&device)
            || !["fp16", "bf16", "fp32"].contains(&precision)
            || (device == "cpu" && precision != "fp32")
        {
            return Err(AiError::invalid(
                "unsupported SAM 2 model or runtime setting",
            ));
        }
        Ok(Self {
            session: Session::new(base)?,
            model: model.into(),
            device: device.into(),
            precision: precision.into(),
        })
    }
    pub fn verify_server(&self) -> Result<(), AiError> {
        self.session.verify_server()
    }
    pub fn prepare(&mut self, image: &[u8]) -> Result<(), AiError> {
        self.session.prepare(image, SAM2_NODES, "craft-sam2", true)
    }
    pub fn ground(&self, text: &str) -> Result<Vec<u8>, AiError> {
        let p = self
            .session
            .prepared
            .as_ref()
            .ok_or_else(|| AiError::invalid("selection image not prepared"))?;
        let graph = json!({
            "1":{"class_type":"LoadImage","inputs":{"image":p.filename}},
            "2":{"class_type":"Florence2ModelLoader","inputs":{"model":crate::detector::FLORENCE2_DETECTOR,"precision":"fp16","convert_to_safetensors":false}},
            "3":{"class_type":"Florence2Run","inputs":{"image":["1",0],"florence2_model":["2",0],"text_input":text,"task":"caption_to_phrase_grounding","fill_mask":true,"keep_model_loaded":true}},
            "4":{"class_type":"Florence2toCoordinates","inputs":{"data":["3",3],"index":"","batch":false}},
            "5":{"class_type":"DownloadAndLoadSAM2Model","inputs":{"model":self.model,"segmentor":"single_image","device":self.device,"precision":self.precision}},
            "6":{"class_type":"Sam2Segmentation","inputs":{"sam2_model":["5",0],"image":["1",0],"keep_model_loaded":true,"bboxes":["4",1],"individual_objects":false}},
            "7":{"class_type":"MaskToImage","inputs":{"mask":["6",0]}},
            "8":{"class_type":"SaveImage","inputs":{"images":["7",0],"filename_prefix":"craft-sam2-selection"}}
        });
        self.session.run(graph, text)
    }
}

/// GroundingDINO detection followed by the explicitly selected SAM 1 checkpoint. Calls block; use a worker.
pub struct Sam1Session {
    session: Session,
    sam_model: String,
    grounding_model: String,
}
impl Sam1Session {
    pub fn new(base: &str, sam_model: &str, grounding_model: &str) -> Result<Self, AiError> {
        let sam = [
            "sam_vit_h (2.56GB)",
            "sam_vit_l (1.25GB)",
            "sam_vit_b (375MB)",
            "sam_hq_vit_h (2.57GB)",
            "sam_hq_vit_l (1.25GB)",
            "sam_hq_vit_b (379MB)",
            "mobile_sam(39MB)",
        ];
        let dino = [
            "GroundingDINO_SwinT_OGC (694MB)",
            "GroundingDINO_SwinB (938MB)",
        ];
        if !sam.contains(&sam_model) || !dino.contains(&grounding_model) {
            return Err(AiError::invalid("unsupported SAM 1 or GroundingDINO model"));
        }
        Ok(Self {
            session: Session::new(base)?,
            sam_model: sam_model.into(),
            grounding_model: grounding_model.into(),
        })
    }
    pub fn verify_server(&self) -> Result<(), AiError> {
        self.session.verify_server()
    }
    pub fn prepare(&mut self, image: &[u8]) -> Result<(), AiError> {
        self.session.prepare(image, SAM1_NODES, "craft-sam1", false)
    }
    pub fn ground(&self, text: &str) -> Result<Vec<u8>, AiError> {
        let p = self
            .session
            .prepared
            .as_ref()
            .ok_or_else(|| AiError::invalid("selection image not prepared"))?;
        let graph = json!({
            "1":{"class_type":"LoadImage","inputs":{"image":p.filename}},
            "2":{"class_type":"SAMModelLoader (segment anything)","inputs":{"model_name":self.sam_model}},
            "3":{"class_type":"GroundingDinoModelLoader (segment anything)","inputs":{"model_name":self.grounding_model}},
            "4":{"class_type":"GroundingDinoSAMSegment (segment anything)","inputs":{"sam_model":["2",0],"grounding_dino_model":["3",0],"image":["1",0],"prompt":text,"threshold":0.3}},
            "5":{"class_type":"MaskToImage","inputs":{"mask":["4",1]}},
            "8":{"class_type":"SaveImage","inputs":{"images":["5",0],"filename_prefix":"craft-sam1-selection"}}
        });
        self.session.run(graph, text)
    }
}
