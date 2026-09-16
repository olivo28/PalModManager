use super::types::*;
use regex::Regex;

/// Extracts the inner text of the first matching XML tag e.g. <Name>Text</Name>
fn extract_tag_text(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{}\b[^>]*>(.*?)</{}>", regex::escape(tag), regex::escape(tag));
    Regex::new(&pattern).ok()?.captures(xml).map(|cap| cap[1].trim().to_string())
}

/// Extracts an attribute value from an XML tag string e.g. path="foo/bar"
fn extract_attr(tag_str: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"(?i)\b{}\s*=\s*["']([^"']*)["']"#, regex::escape(attr));
    Regex::new(&pattern).ok()?.captures(tag_str).map(|cap| cap[1].trim().to_string())
}

/// Extracts all matching tag blocks, returning (full_tag_with_attrs, inner_content)
fn extract_tag_blocks(xml: &str, tag: &str) -> Vec<(String, String)> {
    let pattern = format!(r"(?is)<({}\b[^>]*)>(.*?)</{}>", regex::escape(tag), regex::escape(tag));
    let Ok(re) = Regex::new(&pattern) else { return Vec::new(); };
    re.captures_iter(xml)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .collect()
}

/// Extracts self-closing or paired tags for <file> and <folder>
fn parse_file_entries(xml: &str) -> Vec<FomodFileEntry> {
    let mut entries = Vec::new();
    let re_tags = Regex::new(r#"(?is)<(file|folder)\b([^>]*?)(?:/>|>)"#).unwrap();

    for cap in re_tags.captures_iter(xml) {
        let tag_type = cap[1].to_lowercase();
        let attrs = &cap[2];
        let is_folder = tag_type == "folder";

        let source = extract_attr(attrs, "source").unwrap_or_default();
        let destination = extract_attr(attrs, "destination").unwrap_or_default();
        let priority = extract_attr(attrs, "priority")
            .and_then(|p| p.parse::<i32>().ok())
            .unwrap_or(0);

        if !source.is_empty() {
            entries.push(FomodFileEntry {
                source,
                destination,
                priority,
                is_folder,
            });
        }
    }
    entries
}

/// Parses visibility / flagDependency conditions
fn parse_visibility(xml: &str) -> Option<FomodVisibility> {
    let vis_blocks = extract_tag_blocks(xml, "visible");
    let (vis_attrs, vis_content) = if let Some((attrs, content)) = vis_blocks.first() {
        (attrs.as_str(), content.as_str())
    } else {
        return None;
    };

    let operator = extract_attr(vis_attrs, "operator")
        .or_else(|| {
            let dep_blocks = extract_tag_blocks(vis_content, "dependencies");
            dep_blocks.first().and_then(|(d_attrs, _)| extract_attr(d_attrs, "operator"))
        })
        .unwrap_or_else(|| "And".to_string());
    let mut flag_deps = Vec::new();

    let re_dep = Regex::new(r#"(?is)<flagDependency\b([^>]*?)(?:/>|>(.*?)</flagDependency>)"#).unwrap();
    for cap in re_dep.captures_iter(vis_content) {
        let attrs = &cap[1];
        let flag = extract_attr(attrs, "flag").unwrap_or_default();
        let value = extract_attr(attrs, "value").unwrap_or_default();
        if !flag.is_empty() {
            flag_deps.push(FomodFlagDependency { flag, value });
        }
    }

    Some(FomodVisibility {
        operator,
        flag_dependencies: flag_deps,
    })
}

/// Parses info.xml metadata
pub fn parse_fomod_info(xml: &str) -> FomodInfo {
    let xml = xml.trim_start_matches('\u{feff}');
    FomodInfo {
        name: extract_tag_text(xml, "Name").unwrap_or_default(),
        author: extract_tag_text(xml, "Author").unwrap_or_default(),
        version: extract_tag_text(xml, "Version").unwrap_or_default(),
        description: extract_tag_text(xml, "Description").unwrap_or_default(),
        website: extract_tag_text(xml, "Website").unwrap_or_default(),
    }
}

/// Parses ModuleConfig.xml
pub fn parse_fomod_config(xml: &str) -> Result<FomodConfig, String> {
    let xml = xml.trim_start_matches('\u{feff}');
    let module_name = extract_tag_text(xml, "moduleName").unwrap_or_else(|| "Mod".to_string());

    let module_image = {
        let re_img = Regex::new(r#"(?is)<moduleImage\b([^>]*?)(?:/>|>(.*?)</moduleImage>)"#).unwrap();
        re_img.captures(xml).and_then(|cap| extract_attr(&cap[1], "path"))
    };

    // 1. Required Install Files
    let mut required_install_files = Vec::new();
    for (_attrs, content) in extract_tag_blocks(xml, "requiredInstallFiles") {
        required_install_files.extend(parse_file_entries(&content));
    }

    // 2. Install Steps
    let mut install_steps = Vec::new();
    let re_steps = Regex::new(r#"(?is)<installStep\b([^>]*)>(.*?)</installStep>"#).unwrap();

    let mut plugin_counter = 0;

    for cap_step in re_steps.captures_iter(xml) {
        let step_attrs = &cap_step[1];
        let step_content = &cap_step[2];

        let step_name = extract_attr(step_attrs, "name").unwrap_or_else(|| format!("Step {}", install_steps.len() + 1));
        let visible = parse_visibility(step_content);

        let mut groups = Vec::new();
        let re_groups = Regex::new(r#"(?is)<group\b([^>]*)>(.*?)</group>"#).unwrap();

        for cap_group in re_groups.captures_iter(step_content) {
            let group_attrs = &cap_group[1];
            let group_content = &cap_group[2];

            let group_name = extract_attr(group_attrs, "name").unwrap_or_else(|| "Options".to_string());
            let group_type = extract_attr(group_attrs, "type").unwrap_or_else(|| "SelectAny".to_string());

            let mut plugins = Vec::new();
            let re_plugins = Regex::new(r#"(?is)<plugin\b([^>]*)>(.*?)</plugin>"#).unwrap();

            for cap_plug in re_plugins.captures_iter(group_content) {
                let plug_attrs = &cap_plug[1];
                let plug_content = &cap_plug[2];

                plugin_counter += 1;
                let plugin_id = format!("plugin_{}", plugin_counter);
                let plugin_name = extract_attr(plug_attrs, "name").unwrap_or_else(|| format!("Option {}", plugins.len() + 1));
                let description = extract_tag_text(plug_content, "description").unwrap_or_default();

                let image = {
                    let re_p_img = Regex::new(r#"(?is)<image\b([^>]*?)(?:/>|>(.*?)</image>)"#).unwrap();
                    re_p_img.captures(plug_content).and_then(|c| extract_attr(&c[1], "path"))
                };

                let type_descriptor = {
                    let re_t = Regex::new(r#"(?is)<type\b([^>]*?)(?:/>|>(.*?)</type>)"#).unwrap();
                    re_t.captures(plug_content)
                        .and_then(|c| extract_attr(&c[1], "name"))
                        .unwrap_or_else(|| "Optional".to_string())
                };

                // Files within plugin
                let mut files = Vec::new();
                for (_f_attrs, f_content) in extract_tag_blocks(plug_content, "files") {
                    files.extend(parse_file_entries(&f_content));
                }

                // Condition flags set by this plugin
                let mut condition_flags = Vec::new();
                for (_cf_attrs, cf_content) in extract_tag_blocks(plug_content, "conditionFlags") {
                    let re_flag = Regex::new(r#"(?is)<flag\b([^>]*)>(.*?)</flag>"#).unwrap();
                    for cap_f in re_flag.captures_iter(&cf_content) {
                        let f_attrs = &cap_f[1];
                        let f_val = cap_f[2].trim().to_string();
                        if let Some(f_name) = extract_attr(f_attrs, "name") {
                            condition_flags.push(FomodFlag {
                                name: f_name,
                                value: f_val,
                            });
                        }
                    }
                }

                plugins.push(FomodPlugin {
                    id: plugin_id,
                    name: plugin_name,
                    description,
                    image,
                    image_base64: None,
                    type_descriptor,
                    files,
                    condition_flags,
                });
            }

            groups.push(FomodGroup {
                name: group_name,
                group_type,
                plugins,
            });
        }

        install_steps.push(FomodStep {
            name: step_name,
            visible,
            groups,
        });
    }

    // 3. Conditional File Installs (Matrix)
    let mut conditional_file_installs = Vec::new();
    for (_pats_attrs, pats_content) in extract_tag_blocks(xml, "conditionalFileInstalls") {
        let re_pat = Regex::new(r#"(?is)<pattern\b[^>]*>(.*?)</pattern>"#).unwrap();
        for cap_p in re_pat.captures_iter(&pats_content) {
            let pat_content = &cap_p[1];

            // Dependencies
            let mut dep_vis = FomodVisibility::default();
            for (d_attrs, d_content) in extract_tag_blocks(pat_content, "dependencies") {
                dep_vis.operator = extract_attr(&d_attrs, "operator").unwrap_or_else(|| "And".to_string());
                let re_dep = Regex::new(r#"(?is)<flagDependency\b([^>]*?)(?:/>|>(.*?)</flagDependency>)"#).unwrap();
                for cap_d in re_dep.captures_iter(&d_content) {
                    let attrs = &cap_d[1];
                    let flag = extract_attr(attrs, "flag").unwrap_or_default();
                    let value = extract_attr(attrs, "value").unwrap_or_default();
                    if !flag.is_empty() {
                        dep_vis.flag_dependencies.push(FomodFlagDependency { flag, value });
                    }
                }
            }

            // Files
            let mut pat_files = Vec::new();
            for (_pf_attrs, pf_content) in extract_tag_blocks(pat_content, "files") {
                pat_files.extend(parse_file_entries(&pf_content));
            }

            conditional_file_installs.push(FomodPattern {
                dependencies: dep_vis,
                files: pat_files,
            });
        }
    }

    Ok(FomodConfig {
        module_name,
        module_image,
        info: None,
        required_install_files,
        install_steps,
        conditional_file_installs,
        banner_base64: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fomod_info() {
        let xml = r#"<fomod>
            <Name>Better Base Building</Name>
            <Author>OkaRei</Author>
            <Version>1.0.0</Version>
            <Description>Modifies base building boundaries</Description>
            <Website>https://nexusmods.com/palworld/mods/5544</Website>
        </fomod>"#;

        let info = parse_fomod_info(xml);
        assert_eq!(info.name, "Better Base Building");
        assert_eq!(info.author, "OkaRei");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.description, "Modifies base building boundaries");
        assert_eq!(info.website, "https://nexusmods.com/palworld/mods/5544");
    }

    #[test]
    fn test_parse_fomod_config_steps_and_flags() {
        let xml = r#"<config>
            <moduleName>Sample Mod</moduleName>
            <requiredInstallFiles>
                <file source="main.pak" destination="Pal/Content/Paks/~mods" priority="10" />
            </requiredInstallFiles>
            <installSteps order="Explicit">
                <installStep name="Settings">
                    <group name="Variant Choice" type="SelectExactlyOne">
                        <plugins>
                            <plugin name="Option A">
                                <description>First option</description>
                                <typeDescriptor>
                                    <type name="Recommended" />
                                </typeDescriptor>
                                <files>
                                    <file source="opt_a.pak" destination="Pal/Content/Paks/~mods" />
                                </files>
                                <conditionFlags>
                                    <flag name="Variant">A</flag>
                                </conditionFlags>
                            </plugin>
                        </plugins>
                    </group>
                </installStep>
            </installSteps>
        </config>"#;

        let config = parse_fomod_config(xml).expect("parse failed");
        assert_eq!(config.module_name, "Sample Mod");
        assert_eq!(config.required_install_files.len(), 1);
        assert_eq!(config.required_install_files[0].source, "main.pak");
        assert_eq!(config.install_steps.len(), 1);

        let step = &config.install_steps[0];
        assert_eq!(step.name, "Settings");
        assert_eq!(step.groups.len(), 1);

        let group = &step.groups[0];
        assert_eq!(group.name, "Variant Choice");
        assert_eq!(group.group_type, "SelectExactlyOne");
        assert_eq!(group.plugins.len(), 1);

        let plugin = &group.plugins[0];
        assert_eq!(plugin.name, "Option A");
        assert_eq!(plugin.type_descriptor, "Recommended");
        assert_eq!(plugin.condition_flags.len(), 1);
        assert_eq!(plugin.condition_flags[0].name, "Variant");
        assert_eq!(plugin.condition_flags[0].value, "A");
    }

    #[test]
    fn test_parse_real_better_base_building_xml() {
        let zip_path = "C:/Users/Antikux/Downloads/BetterBaseBuilding 5544 2.3.zip";
        if std::path::Path::new(zip_path).exists() {
            let xml_opt = crate::zip_handler::read_archive_file(zip_path, "fomod/ModuleConfig.xml");
            assert!(xml_opt.is_some(), "read_archive_file failed on UTF-16LE ModuleConfig.xml");
            let xml = xml_opt.unwrap();
            let res = parse_fomod_config(&xml);
            assert!(res.is_ok(), "Failed to parse: {:?}", res.err());
            let config = res.unwrap();
            assert_eq!(config.install_steps.len(), 6);
            assert_eq!(config.required_install_files.len(), 1);
            assert_eq!(config.conditional_file_installs.len(), 1);

            let info_opt = crate::zip_handler::read_archive_file(zip_path, "fomod/info.xml");
            assert!(info_opt.is_some(), "read_archive_file failed on UTF-16LE info.xml");
            let info = parse_fomod_info(&info_opt.unwrap());
            assert_eq!(info.name, "BetterBaseBuilding");
            assert_eq!(info.version, "2.3");
        }
    }
}


