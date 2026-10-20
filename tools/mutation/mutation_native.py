"""Deliberate-bug cases for group `nf`: planning and offering a built-in runtime's own files.

Loaded by mutation_check.py. Each case breaks one rule and names the test that should notice.
Format: (name, file, old, new, package, test target, test filter).
"""
F = 'crates/ai-features/src/'
FE = 'craft-ai-features'

NF = [
    ('native-plan-ignores-usable', F + 'plan.rs', 'let Some(route) = recipe.filter(|r| r.usable).and_then(|r| {', 'let Some(route) = recipe.and_then(|r| {', FE, 'native_setup', 'cannot_run_yet'),
    ('native-plan-takes-a-route-without-files', F + 'plan.rs', '.find(|n| n.provider == provider && !n.models.is_empty())\n    }) else {\n        return out;\n    };\n    fill_files(&mut out, &route.models, inventory);\n    check_disk', '.find(|n| n.provider == provider)\n    }) else {\n        return out;\n    };\n    fill_files(&mut out, &route.models, inventory);\n    check_disk', FE, 'native_setup', 'unfinished_or_unknown'),
    ('native-plan-takes-any-provider', F + 'plan.rs', '.find(|n| n.provider == provider && !n.models.is_empty())\n    }) else {\n        return out;\n    };\n    fill_files(&mut out, &route.models, inventory);\n    check_disk', '.find(|n| !n.models.is_empty())\n    }) else {\n        return out;\n    };\n    fill_files(&mut out, &route.models, inventory);\n    check_disk', FE, 'native_setup', 'unfinished_or_unknown'),
    ('native-plan-skips-the-disk-check', F + 'plan.rs', '    fill_files(&mut out, &route.models, inventory);\n    check_disk(&mut out, inventory);', '    fill_files(&mut out, &route.models, inventory);', FE, 'native_setup', 'free_space'),
    ('native-offer-ignores-the-provider', F + 'setup.rs', 'selected_provider.is_none_or(|provider| n.provider == provider)', 'true', FE, 'setup', 'selected_provider'),
    ('built-in-offer-compares-comfyui-files', F + 'setup.rs', 'let compared = if let Some(native) = native {', 'let compared = if let Some(native) = native.filter(|_| false) {', FE, 'setup', 'needs_only_the_model'),
    ('option-offered-for-a-route-without-files', F + 'options.rs', 'for native in recipe.native.iter().filter(|n| !n.models.is_empty()) {', 'for native in recipe.native.iter() {', FE, 'native_setup', 'not_unbuilt_burn'),
]
