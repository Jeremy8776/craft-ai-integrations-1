//! The built-in feature list.
//!
//! Local recipes are only listed where the source, size and checksum were checked against the
//! real file. A feature with no recipe says so, and the screen shows it as not installable yet.
//! Cloud options are only listed where an adapter exists in this workspace; none of them has been
//! run against its real service.

use crate::feature::{AppStatus, AppUse, Feature};
use crate::recipe::{
    CloudOption, Component, Licence, LocalRecipe, ModelFile, NativeRoute, NodePack, Source,
};
use craft_ai_core::{CapabilityId, InputRole};

fn uses(list: &[(&str, &str)]) -> Vec<AppUse> {
    list.iter()
        .map(|(app, tool)| AppUse {
            app: (*app).to_owned(),
            tool: (*tool).to_owned(),
            status: AppStatus::Proposed,
        })
        .collect()
}

fn cloud(preset: &str, label: &str, note: &str) -> CloudOption {
    CloudOption {
        preset: preset.to_owned(),
        label: label.to_owned(),
        note: note.to_owned(),
    }
}

fn feature(id: &str, title: &str, summary: &str, capability: &str) -> Option<Feature> {
    Some(Feature {
        id: id.to_owned(),
        title: title.to_owned(),
        summary: summary.to_owned(),
        capability: CapabilityId::parse(capability).ok()?,
        inputs: Vec::new(),
        uses: Vec::new(),
        locals: Vec::new(),
        cloud: Vec::new(),
    })
}

/// Meta's SAM 3 checkpoint in the Hugging Face layout, at a pinned revision, with the tokenizer
/// files the text prompt needs. All three sit in one folder, which is how the loader finds them.
fn sam3_transformers_files() -> Vec<ModelFile> {
    const REVISION: &str = "3c879f39826c281e95690f02c7821c4de09afae7";
    let file = |name: &str, size_bytes: u64, sha256: &str| {
        ModelFile {
        folder: "sam3_transformers".to_owned(),
        filename: name.to_owned(),
        size_bytes,
        sha256: sha256.to_owned(),
        source: Source::HuggingFace {
            repo: "facebook/sam3".to_owned(),
            file: name.to_owned(),
            revision: Some(REVISION.to_owned()),
        },
        licence: Licence {
            name: "Meta SAM License".to_owned(),
            url: Some("https://huggingface.co/facebook/sam3".to_owned()),
            accept_required: true,
            note: Some(
                "Meta's release is gated: downloading needs a Hugging Face account that has been granted access."
                    .to_owned(),
            ),
        },
    }
    };
    vec![
        file(
            "model.safetensors",
            3_439_938_512,
            "6d06f0a5f84e435071fe6603e61d0b4cc7b40e0d39d487cfd4d67d8cc11cc14a",
        ),
        file(
            "vocab.json",
            862_328,
            "5047b556ce86ccaf6aa22b3ffccfc52d391ea4accdab9c2f2407da5b742d4363",
        ),
        file(
            "merges.txt",
            524_619,
            "9fd691f7c8039210e0fced15865466c65820d09b63988b0174bfe25de299051a",
        ),
    ]
}

fn sam3() -> LocalRecipe {
    LocalRecipe {
        label: "SAM 3".to_owned(),
        // PhotoCraft runs it today (Object Selection, `Sam3Session` through ComfyUI).
        usable: true,
        workflow: "sam3-segment".to_owned(),
        models: vec![ModelFile {
            folder: "sam3".to_owned(),
            filename: "sam3.safetensors".to_owned(),
            size_bytes: 3_449_860_632,
            sha256: "127037a7a11169c63a210b8e3e9caad24a66abdd65d3a78bbc3a7d8577d57026".to_owned(),
            source: Source::HuggingFace {
                repo: "apozz/sam3-safetensors".to_owned(),
                file: "sam3.safetensors".to_owned(),
                revision: Some("e88da2ed02aded4006f4711ee6a9c75d5aa38d0c".to_owned()),
            },
            licence: Licence {
                name: "Meta SAM License, community copy".to_owned(),
                url: Some("https://huggingface.co/facebook/sam3".to_owned()),
                accept_required: true,
                note: Some(
                    "This file is a community copy of Meta's SAM 3 weights and declares no licence of its own. Meta's release is gated behind the SAM License."
                        .to_owned(),
                ),
            },
        }],
        packs: vec![NodePack {
            label: "ComfyUI-SAM3".to_owned(),
            repo_url: "https://github.com/PozzettiAndrea/ComfyUI-SAM3".to_owned(),
            folder_names: vec!["comfyui-sam3".to_owned()],
        }],
        min_vram_mb: None,
        // The built-in runtimes are ports of Hugging Face's implementation and read that
        // checkpoint layout, which is a different file from the ComfyUI one above (different
        // tensor names, plus the tokenizer files). Burn is a listed slot with no code yet, so
        // apps report it as unavailable.
        native: vec![
            NativeRoute {
                provider: "candle".to_owned(),
                model: "sam3".to_owned(),
                label: "SAM 3, built in (Apple graphics, otherwise the processor)".to_owned(),
                models: sam3_transformers_files(),
            },
            NativeRoute {
                provider: "burn".to_owned(),
                model: "sam3".to_owned(),
                // No Burn code exists yet, so no files are named for it: a route that cannot run must not
                // offer a 3.4 GB download.
                label: "SAM 3, built in (any graphics card)".to_owned(),
                models: Vec::new(),
            },
        ],
        components: Vec::new(),
    }
}

/// Meta's SAM 2 checkpoint in the layout the `ComfyUI-segment-anything-2` node pack reads, at a
/// pinned revision of a community conversion repo. SAM 2 has no text understanding, so the words
/// of a prompt are found by Microsoft's Florence-2-large detector, whose files are set up here too.
fn sam2(label: &str, workflow: &str, filename: &str, size_bytes: u64, sha256: &str) -> LocalRecipe {
    let mut recipe = LocalRecipe {
        label: label.to_owned(),
        // No adapter runs these generations yet: flip when the app route executes them.
        usable: false,
        workflow: workflow.to_owned(),
        models: vec![ModelFile {
            folder: "sam2".to_owned(),
            filename: filename.to_owned(),
            size_bytes,
            sha256: sha256.to_owned(),
            source: Source::HuggingFace {
                repo: "Kijai/sam2-safetensors".to_owned(),
                file: filename.to_owned(),
                revision: Some("f885607d88bb3f9145efa49c3e3c50a9e5bf13eb".to_owned()),
            },
            licence: Licence {
                name: "Apache-2.0, community conversion".to_owned(),
                url: Some("https://github.com/facebookresearch/sam2".to_owned()),
                accept_required: false,
                note: Some(
                    "This file is a community conversion of Meta's SAM 2 weights, which Meta releases under Apache 2.0."
                        .to_owned(),
                ),
            },
        }],
        packs: vec![
            NodePack {
                label: "ComfyUI segment anything 2".to_owned(),
                repo_url: "https://github.com/kijai/ComfyUI-segment-anything-2".to_owned(),
                folder_names: vec!["comfyui-segment-anything-2".to_owned()],
            },
            NodePack {
                label: "ComfyUI Florence 2".to_owned(),
                repo_url: "https://github.com/kijai/ComfyUI-Florence2".to_owned(),
                folder_names: vec!["comfyui-florence2".to_owned()],
            },
        ],
        min_vram_mb: None,
        native: Vec::new(),
        components: Vec::new(),
    };
    recipe.models.extend(florence2_large_files());
    recipe
}

/// Microsoft's Florence-2-large, the detector that turns the words of a prompt into boxes for SAM 2
/// and SAM 2.1. It is MIT licensed, and every file is pinned to one revision and checked by
/// SHA-256. ComfyUI's Florence 2 add-on lists the folders of `models/LLM`, so the files sit in
/// `LLM/Florence-2-large/`.
fn florence2_large_files() -> Vec<ModelFile> {
    const REVISION: &str = "21a599d414c4d928c9032694c424fb94458e3594";
    let file = |leaf: &str, size_bytes: u64, sha256: &str| {
        ModelFile {
        folder: "LLM".to_owned(),
        filename: format!("{FLORENCE2_LARGE_DIR}/{leaf}"),
        size_bytes,
        sha256: sha256.to_owned(),
        source: Source::HuggingFace {
            repo: "microsoft/Florence-2-large".to_owned(),
            file: leaf.to_owned(),
            revision: Some(REVISION.to_owned()),
        },
        licence: Licence {
            name: "MIT".to_owned(),
            url: Some(format!(
                "https://huggingface.co/microsoft/Florence-2-large/blob/{REVISION}/LICENSE"
            )),
            accept_required: false,
            note: Some(
                "Published by Microsoft. The folder also holds Python files that ComfyUI's Florence 2 add-on loads and runs; they are pinned to one revision and checked against SHA-256."
                    .to_owned(),
            ),
        },
    }
    };
    vec![
        file(
            "model.safetensors",
            1_553_563_458,
            "4f38ce741c6b71188fe2b3419a55e11917a8a7b321ae2e63c61da0191b0ebad7",
        ),
        file(
            "config.json",
            2_445,
            "6f8a8f92a74ce18b5c1e5646b4a8222477dd1ac49f31b953e75d6fc3e0f8583a",
        ),
        file(
            "configuration_florence2.py",
            15_119,
            "de2e45a975b3582de05d2f4d963a3e9f9a3d20dccf78d28e0052932a0be93bdf",
        ),
        file(
            "generation_config.json",
            51,
            "30e9865458ecc8ee931eeeb43f44f1d169c5ab95be39e0072142a7a6b8f31990",
        ),
        file(
            "modeling_florence2.py",
            127_455,
            "5162bf465e61b6e29cc113a467630ec3cb56ed8e4d46eb6207157f10fb9b8a24",
        ),
        file(
            "preprocessor_config.json",
            806,
            "2f5921bbc53c7cc04251e1027b45b1cec726276be6db23d1bb40641bfbe2cf29",
        ),
        file(
            "processing_florence2.py",
            48_674,
            "c655782a9e4347965c735ea54cbc4e98fdbc02155ffd1ce2ecd61f42c45eda28",
        ),
        file(
            "tokenizer.json",
            1_355_863,
            "847bbeab6174d66a88898f729d52fa8d355fafe1bea101cf960dd404581df70e",
        ),
        file(
            "tokenizer_config.json",
            34,
            "79ffcf43af8ebda99d165f61d243180da2e2639952e41e71e11611c18770489c",
        ),
        file(
            "vocab.json",
            1_099_884,
            "394fdc63c71aabe0a9b97117f5d62fb5fcc4d59b2b3ea929a3929e6a53217b3c",
        ),
    ]
}

/// The folder name ComfyUI's Florence 2 add-on shows for the detector, and the graph asks for.
pub const FLORENCE2_LARGE_DIR: &str = "Florence-2-large";

fn sam21() -> LocalRecipe {
    // The node pack loads SAM 2.1 at half precision by default, which reads the `-fp16` file.
    let mut recipe = sam2(
        "SAM 2.1",
        "sam21-segment",
        "sam2.1_hiera_large-fp16.safetensors",
        448_999_828,
        "278af5296fcc153aa94645a4981e93fe8b7a936a9fa27dbd71e8c850df6d7038",
    );
    // PhotoCraft now runs this selected text-grounded subject path.
    recipe.usable = true;
    recipe.components = sam21_components();
    recipe
}

/// What the SAM 2.1 text-grounded path runs besides its checkpoint, with each publisher's own
/// statement as read on 20/10/2026. Disclosure only: see [`Component`].
fn sam21_components() -> Vec<Component> {
    let declared = |name: &str, url: &str, note: Option<&str>| Licence {
        name: name.to_owned(),
        url: Some(url.to_owned()),
        accept_required: false,
        note: note.map(str::to_owned),
    };
    vec![
        Component {
            label: "SAM 2.1 checkpoint".to_owned(),
            role: "Cuts the subject out".to_owned(),
            licence: declared(
                "Apache-2.0, as declared by Meta and by the conversion's publisher",
                "https://github.com/facebookresearch/sam2/blob/main/LICENSE",
                None,
            ),
        },
        Component {
            label: "ComfyUI segment anything 2 add-on".to_owned(),
            role: "Runs SAM 2.1 inside ComfyUI".to_owned(),
            licence: declared(
                "Apache-2.0, as declared by its publisher",
                "https://github.com/kijai/ComfyUI-segment-anything-2/blob/main/LICENSE",
                Some(
                    "If the checkpoint file is missing, this add-on downloads it from Hugging Face by itself. ArtCraft checks the file before it lets a Craft app use the model.",
                ),
            ),
        },
        Component {
            label: "ComfyUI Florence 2 add-on".to_owned(),
            role: "Runs the detector inside ComfyUI".to_owned(),
            licence: declared(
                "MIT, as declared by its publisher",
                "https://github.com/kijai/ComfyUI-Florence2/blob/main/LICENSE",
                None,
            ),
        },
        Component {
            label: "Florence-2-large detector".to_owned(),
            role: "Finds the words you type in the picture".to_owned(),
            licence: declared(
                "MIT, as declared by Microsoft",
                "https://huggingface.co/microsoft/Florence-2-large/blob/main/LICENSE",
                Some(
                    "ArtCraft sets this one up. You can run another Florence-2 variant in ComfyUI yourself; its terms are its publisher's, and this list does not cover it.",
                ),
            ),
        },
    ]
}

fn sam20() -> LocalRecipe {
    sam2(
        "SAM 2",
        "sam2-segment",
        "sam2_hiera_large.safetensors",
        897_827_480,
        "ad781e6f18af5d7523075642f5da2d2b874cec6b16cebfd4f6ea1c6a289d9605",
    )
}

/// Meta's original SAM checkpoint plus the GroundingDINO text detector that turns words into
/// boxes for it, and the language model files that detector reads. All three sit in ComfyUI
/// model folders the `comfyui_segment_anything` node pack scans.
fn sam1() -> LocalRecipe {
    const DINO_REVISION: &str = "a94c9b567a2a374598f05c584e96798a170c56fb";
    const BERT_REVISION: &str = "86b5e0934494bd15c9632b12f734a8a67f723594";
    let apache = |note: &str| Licence {
        name: "Apache-2.0".to_owned(),
        url: Some("https://www.apache.org/licenses/LICENSE-2.0".to_owned()),
        accept_required: false,
        note: Some(note.to_owned()),
    };
    let dino_file = |filename: &str, size_bytes: u64, sha256: &str| ModelFile {
        folder: "grounding-dino".to_owned(),
        filename: filename.to_owned(),
        size_bytes,
        sha256: sha256.to_owned(),
        source: Source::HuggingFace {
            repo: "ShilongLiu/GroundingDINO".to_owned(),
            file: filename.to_owned(),
            revision: Some(DINO_REVISION.to_owned()),
        },
        licence: apache("GroundingDINO is released by its authors under Apache 2.0."),
    };
    let bert_file = |filename: &str, size_bytes: u64, sha256: &str| ModelFile {
        folder: "bert-base-uncased".to_owned(),
        filename: filename.to_owned(),
        size_bytes,
        sha256: sha256.to_owned(),
        source: Source::HuggingFace {
            repo: "google-bert/bert-base-uncased".to_owned(),
            file: filename.to_owned(),
            revision: Some(BERT_REVISION.to_owned()),
        },
        licence: apache("The BERT language model files are released by Google under Apache 2.0."),
    };
    LocalRecipe {
        label: "SAM 1".to_owned(),
        usable: false,
        workflow: "sam1-segment".to_owned(),
        models: vec![
            ModelFile {
                folder: "sams".to_owned(),
                filename: "sam_vit_b_01ec64.pth".to_owned(),
                size_bytes: 375_042_383,
                sha256: "ec2df62732614e57411cdcf32a23ffdf28910380d03139ee0f4fcbe91eb8c912"
                    .to_owned(),
                source: Source::Url {
                    url: "https://dl.fbaipublicfiles.com/segment_anything/sam_vit_b_01ec64.pth"
                        .to_owned(),
                },
                licence: apache(
                    "Meta's original SAM checkpoint, served from Meta's own file server.",
                ),
            },
            dino_file(
                "groundingdino_swint_ogc.pth",
                693_997_677,
                "3b3ca2563c77c69f651d7bd133e97139c186df06231157a64c507099c52bc799",
            ),
            dino_file(
                "GroundingDINO_SwinT_OGC.cfg.py",
                1_006,
                "172e80017f9395668a9cb5d1b8bd9d061c0e360471c6ed673c83b69bb14399f1",
            ),
            bert_file(
                "model.safetensors",
                440_449_768,
                "68d45e234eb4a928074dfd868cead0219ab85354cc53d20e772753c6bb9169d3",
            ),
            bert_file(
                "config.json",
                570,
                "7160e1553ad2ca51d8c1cb066be533db31826e12d173824c1bb0cb1a4f187d20",
            ),
            bert_file(
                "vocab.txt",
                231_508,
                "07eced375cec144d27c900241f3e339478dec958f92fddbc551f295c992038a3",
            ),
            bert_file(
                "tokenizer.json",
                466_062,
                "ce64fce797c24f68df90b40a3f74f579b336a493db14bd583fd520ea0d8c9a98",
            ),
            bert_file(
                "tokenizer_config.json",
                48,
                "a025160ef0431f1a392f6f050c1310f4c5d9fb6f275932dbccba73c4d214bf10",
            ),
        ],
        packs: vec![NodePack {
            label: "ComfyUI segment anything".to_owned(),
            repo_url: "https://github.com/storyicon/comfyui_segment_anything".to_owned(),
            folder_names: vec!["comfyui_segment_anything".to_owned()],
        }],
        min_vram_mb: None,
        native: Vec::new(),
        components: Vec::new(),
    }
}

/// Every feature the launcher offers, in the order it shows them. A test pins the count and
/// every capability id, so the empty list this falls back to cannot ship.
pub fn builtin() -> Vec<Feature> {
    try_builtin().unwrap_or_default()
}

fn try_builtin() -> Option<Vec<Feature>> {
    let mut select = feature(
        "select-subject",
        "Select subject",
        "Pick out the main subject, or anything you describe in words, and turn it into a selection or a mask.",
        "image.segment",
    )?;
    select.uses = uses(&[
        ("PhotoCraft", "Select Subject and Object Selection"),
        ("LightCraft", "Subject, sky and people masks"),
        ("EffectCraft", "Roto and matte"),
    ]);
    select.inputs = vec![InputRole::Source, InputRole::Prompt];
    select.locals = vec![sam3(), sam21(), sam20(), sam1()];

    let mut background = feature(
        "remove-background",
        "Remove background",
        "Cut the subject out and leave the background transparent.",
        "image.matte",
    )?;
    background.inputs = vec![InputRole::Source];
    background.uses = uses(&[("PhotoCraft", "Remove Background")]);
    // No local recipe and no hosted route yet: no adapter here matts an image, and no model file
    // has been checked against its real source. The screen says so instead of offering a guess.

    let mut remove = feature(
        "remove-object",
        "Remove objects",
        "Mark something and the area fills in to match its surroundings.",
        "image.inpaint",
    )?;
    remove.inputs = vec![InputRole::Source, InputRole::Mask];
    remove.uses = uses(&[
        ("PhotoCraft", "Remove tool"),
        ("LightCraft", "Generative remove"),
    ]);

    let mut fill = feature(
        "generative-fill",
        "Generative fill",
        "Select an area, describe what should be there, and get new content that blends in.",
        "image.inpaint",
    )?;
    fill.inputs = vec![InputRole::Source, InputRole::Mask, InputRole::Prompt];
    fill.uses = uses(&[("PhotoCraft", "Generative Fill")]);
    fill.cloud = vec![cloud(
        "openai",
        "OpenAI image editing",
        "Your own OpenAI key, billed by OpenAI. Built and tested offline only.",
    )];

    let mut expand = feature(
        "generative-expand",
        "Generative expand",
        "Make the canvas bigger and fill the new area to match the picture.",
        "image.outpaint",
    )?;
    expand.inputs = vec![InputRole::Source, InputRole::Prompt];
    expand.uses = uses(&[("PhotoCraft", "Generative Expand")]);

    let mut extend = feature(
        "extend-sequence",
        "Extend sequence",
        "Add frames to the end of a clip so the shot carries on.",
        "video.generate",
    )?;
    extend.inputs = vec![InputRole::Source, InputRole::Prompt];
    extend.uses = uses(&[
        ("FilmCraft", "Generative video"),
        ("EffectCraft", "Generative video"),
    ]);

    let mut image = feature(
        "generate-image",
        "Generate images",
        "Make a picture from a description.",
        "image.generate",
    )?;
    image.inputs = vec![InputRole::Prompt];
    image.uses = uses(&[
        ("PhotoCraft", "Generate Image"),
        ("DesignCraft", "Image generation"),
        ("DeckCraft", "Slide images"),
    ]);
    image.cloud = vec![
        cloud("openai", "OpenAI", "Your own OpenAI key, billed by OpenAI."),
        cloud(
            "bfl",
            "Black Forest Labs FLUX",
            "Your own BFL key, billed by Black Forest Labs.",
        ),
        cloud(
            "vertex-express",
            "Google Gemini image (Nano Banana)",
            "Express-mode API key only. The key cannot be tested without a paid request.",
        ),
        cloud(
            "artcraft-omni",
            "ArtCraft",
            "Staff-enabled API key. Whether a subscription covers it is unverified.",
        ),
    ];

    let mut video = feature(
        "generate-video",
        "Generate video",
        "Make a clip from a description.",
        "video.generate",
    )?;
    video.inputs = vec![InputRole::Prompt];
    video.uses = uses(&[
        ("FilmCraft", "Generative video"),
        ("EffectCraft", "Generative video"),
    ]);
    video.cloud = vec![
        cloud(
            "bfl",
            "Black Forest Labs",
            "Your own BFL key, billed by Black Forest Labs.",
        ),
        cloud(
            "seedance",
            "Seedance (BytePlus ModelArk)",
            "Text to video with your BytePlus ModelArk key.",
        ),
        cloud(
            "artcraft-omni",
            "ArtCraft",
            "Staff-enabled API key. Whether a subscription covers it is unverified.",
        ),
    ];

    Some(vec![
        select, background, remove, fill, expand, extend, image, video,
    ])
}
