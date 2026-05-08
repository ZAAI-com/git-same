use super::*;

#[test]
fn parse_pluginkit_output_marks_enabled_extension() {
    let stdout = "+    com.zaai.git-same.Badges(3.1.0)    \
                  /Applications/git-Same.app/Contents/PlugIns/GitSameBadges.appex\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: true,
            enabled: true,
        }
    );
}

#[test]
fn parse_pluginkit_output_marks_disabled_extension() {
    let stdout = "-    com.zaai.git-same.Badges(3.1.0)    \
                  /Applications/git-Same.app/Contents/PlugIns/GitSameBadges.appex\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: true,
            enabled: false,
        }
    );
}

#[test]
fn parse_pluginkit_output_returns_uninstalled_for_empty_stdout() {
    let result = parse_pluginkit_output("", FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: false,
            enabled: false,
        }
    );
}

#[test]
fn parse_pluginkit_output_ignores_other_extensions() {
    let stdout = "+    com.apple.dt.Xcode.SimulatorTrampoline(15.0)    \
                  /Applications/Xcode.app/Contents/PlugIns/SimulatorTrampoline.appex\n\
                  -    com.example.other(1.0)    /Applications/Other.app\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: false,
            enabled: false,
        }
    );
}
