# Phase 07 — TerminalSettingsModel (detailed plan)

## Context Links
- Brief: original placeholder (now expanded into this document).
- WT source: `windows/src/cascadia/TerminalSettingsModel/` (38 `.cpp`, ~21,031 LOC) + `windows/src/cascadia/UnitTests_SettingsModel/` (11 test files, ~12,386 LOC, 158 `TEST_METHOD`).
- Existing Rust port: `crates/settings/src/` (17 files, 3,134 LOC) + `crates/font/src/css_length.rs` (done, line-height + cell-size parser) + `crates/app/src/settings_loader.rs` (thin JSONC loader, ~205 lines, consumed by `page.rs:32`) + `crates/settings/src/editor` skeleton + `crates/control`/`app` read `Profile`/`GlobalAppSettings`/`color`.
- ADRs: ADR-0011 (WinRT `IInspectable` → Rust structs + `Arc` inheritance graph; no trait-object projection; same JSON semantics); ADR-0007 (Azure Cloud Shell generator out of scope); ADR-0001 (per-file ownership phase assignment).
- P01 frozen contract: `settings/src/terminal_settings.rs:27` `TerminalSettings` minimal bundle — P07 may only ADD fields additively.

## WT source map (file:line anchors)
Settings model C++ (`windows/src/cascadia/TerminalSettingsModel/`):
- `CascadiaSettingsSerialization.cpp:1226` `LoadAll()` · `:208` `SettingsLoader::GenerateProfiles()` · `:278` `ApplyRuntimeInitialSettings()` · `:314` `MergeInboxIntoUserSettings()` · `:329` `FindFragmentsAndMergeIntoUserSettings()` · `:826` `_parse()` (GlobalAppSettings::FromJson at `:833`, ColorScheme loop `:837`, Theme loop `:848`, baseLayerProfile `:873`, profile list `:884-898`).
- `CascadiaSettings.cpp:455` `_validateSettings()` · `:476` `_validateAllSchemesExist()` · `:654` `_validateMediaResources()` · `:679` `_validateProfileEnvironmentVariables()` · `:701` `_validateNTMEntries()` · `:727` `_validateRegexes()` · `:945` `_validateKeybindings()` · `:967` `_validateColorSchemesInCommands()` · `:1241` `_validateThemeExists()`. `ApplicationState.cpp:96/117/149/198` ctor/dtor/`Flush`/`_read`/`_write`; `:264` `FromJson`, `:284` `ToJson`; `:310` `AppendPersistedWindowLayout` etc.
- `Profile.cpp:151` `FromJson` · `:170` `LayerJson` · `:327` `ToJson` · `:239` `_FinalizeInheritance()` (parents walk). `Profile.h:108` declares inheritance hooks.
- `GlobalAppSettings.cpp` (562 LOC) — top-level parse/layer/serialize, AddTheme, ExpandCommands.
- `ActionAndArgs.cpp` (419 LOC) — `FromJson`/`GenerateName`/`ToJson` for the action/args union. `ActionArgs.cpp:90-828` ~60 `XxxArgs::GenerateName` impls (`NewTerminalArgs` through `GlobalSummonArgs`).
- `ActionMap.cpp:294` `_FinalizeInheritance` · `:366` `_GetActionByID` · `:410` `AvailableActions` · `:463` `NameMap` · `:550` `_PopulateCumulativeKeyMaps` · `:590` `GlobalHotkeys` · `:599` `KeyBindings` · `:695` `AddAction` · `:840` `GetActionByKeyChord` · `:915` `GetKeyBindingForAction` · `:1041` `ExpandCommands`. `ActionMapSerialization.cpp:1-186` parse/round-trip.
- `Command.cpp:84/93/101` ctor/copy · `:157` `HasName` · `:277` `FromJson` · `:392` `LayerJson` · `:434` `ToJson` · `:505` `ExpandCommands` · `:558` `_expandCommand` · `:813` `LogSettingChanges`.
- `KeyChordSerialization.cpp:325` `FromString` · `:330` `ToString`.
- Generators: `PowershellCoreProfileGenerator.cpp:314` `GenerateProfiles` · `WslDistroGenerator.cpp:241` `GenerateProfiles` · `VisualStudioGenerator.cpp:30` `GenerateProfiles` (delegating to `VsDevCmdGenerator.cpp` / `VsDevShellGenerator.cpp`, fed by `VsSetupConfiguration.cpp:1-126`). `AzureCloudShellGenerator.cpp` — OUT (ADR-0007). `SshHostGenerator.cpp` — feature-flagged off in WT (`Feature_DynamicSSHProfiles`), DELIBERATELY SKIP.
- Inbox: `windows/src/cascadia/TerminalSettingsModel/defaults.json` (P0700 defaults) + `DefaultSettings.h` (already ported as `defaults.rs`).

Test files (`windows/src/cascadia/UnitTests_SettingsModel/`): `DeserializationTests.cpp` (44 methods), `SerializationTests.cpp` (23), `CommandTests.cpp` (8), `KeyBindingsTests.cpp` (14), `ProfileTests.cpp` (11), `ColorSchemeTests.cpp` (7), `TerminalSettingsTests.cpp` (8), `ApplicationStateTests.cpp` (7), `MediaResourceTests.cpp` (23), `NewTabMenuTests.cpp` (2 of larger set), `ThemeTests.cpp` (4 of 11), plus `pch.cpp`.

## Current Rust port status (per file:line)
Done (3,134 LOC, 17 files):
- `defaults.rs:114` `INBOX_SETTINGS_JSON` (verbatim defaults.json), `:132` `USER_DEFAULTS_JSON`, plus all `DefaultSettings.h` constants (font/cols/rows/padding/word-delim/GUIDs).
- `color.rs` (327 LOC) `Color` RGBA + `from_hex`/`campbell_color_table`/`xterm256`.
- `color_scheme.rs:36` `ColorScheme` · `:98` `from_json` (16-color + name validation) · `:118` `layer_json` · `:144` `to_json` · `:172` `set_color_table_entry`. `builtin_schemes.rs` (197 LOC) Campbell/solarized/etc.
- `profile.rs:32` `Profile` (372 LOC) already carries `source:41`, `hidden:43`, `deleted:98`, `orphaned:106`, `updates:90`, `origin:109` — brief's "missing `source`/`hidden`/`deleted`" claim is WRONG (audit correction: those fields exist).
- `profile_serde.rs:29` `from_json` · `:50` `layer_json` (present/null/nullable semantics) · `:37` `to_json`.
- `appearance.rs:17` `Appearance` + `appearance_enums.rs` (228 LOC, CursorShape/IntenseStyle/Stretch/AdjustTextMode). `bell_style.rs` (204 LOC) string-or-bool-or-array parser. `font.rs`/`weight.rs` FontConfig + weight parsing. `theme.rs:9` `ThemeRef` (string-or-object). `profile_enums.rs`/`global_enums.rs` enums. `origin_tag.rs:11` 6-variant enum + JSON round-trip. `global_app_settings.rs:25` `GlobalAppSettings` (defaults match `defaults.json:1-35`). `terminal_settings.rs:27` P01 frozen minimal bundle.
- `crates/app/src/settings_loader.rs:25` thin `CascadiaSettings {profiles, default_profile_guid, globals}` + JSONC `strip_jsonc_comments:107` + `parse_*_from_jsonc` helpers consumed by `page.rs:32`. JSONC parser exists but no fragment/origin/inheritance/validation layering.

TODO / missing (this phase):
1. `ActionMap`/`ActionAndArgs` (~60 args): `ActionAndArgs.cpp`/`ActionArgs.cpp`/`ActionMap.cpp`/`Command.cpp`/`KeyChordSerialization.cpp` — no Rust port at all.
2. `Command` (nested + iterable): `Command.cpp:126/144` nested subcommands, `:558` `_expandCommand` iterable ⊃ no Rust port.
3. Settings loader layering: `CascadiaSettingsSerialization.cpp:1226` `LoadAll` → inbox/fragments/user with `OriginTag` · baseLayerProfile `_FinalizeInheritance` · `_addUserProfileParent` · `MergeInboxIntoUserSettings()` · partial parent graph absent from `settings_loader.rs`.
4. Dynamic profile generators: PowerShell (`PowershellCoreProfileGenerator.cpp:314`), WSL (`WslDistroGenerator.cpp:241`), VS (`VisualStudioGenerator.cpp:30` + VsSetupConfiguration → VsDevCmd/VsDevShell). Out: Azure (ADR-0007); ssh (feature-flagged off in upstream).
5. `ApplicationState` persistence: `ApplicationState.cpp:96-523` file-backed state, window layouts, workspaces, badges — no Rust port.
6. Hot-reload file watcher (debounced): no port (WT uses `winrt::Windows::Storage` watcher on `LocalState`).
7. Validation warnings: `CascadiaSettings.cpp:455-1281` 9 validators + `SettingsLoadWarnings`/`Errors` enums — no Rust port.
8. Legacy migration paths: `MigrateReloadEnvVars` test (`DeserializationTests.cpp:69`) and `RoundtripLegacyToModernActions` (`SerializationTests.cpp:23`) — no Rust port.
9. JSONC parsing: `app/src/settings_loader.rs:107` strips comments but does NOT preserve unknown keys for round-trip write-back (`CascadiaSettingsSerialization.cpp:6.x` `_LayerUserSettings` semantics needed).
10. `TerminalSettings` apply-to-control bridge: `terminal_settings.rs:27` thin bundle missing `color_scheme_name`, `font_fallbacks`, `cursor_height`, `intense_text_style`, `padding`, `background_image*`, `bell_style`, `tab_title`, plus `from(profile, appearance, globals)` resolver — P11 consumers will need these.

## Gaps (concrete WT anchor + Rust gap)
1. ActionMap: `ActionMap.cpp:1-1380` + `ActionMapSerialization.cpp:1-186` + `ActionAndArgs.cpp:1-419` + `ActionArgs.cpp:90-828` (~60 args) + `KeyChordSerialization.cpp:325/330`. Rust: no `crates/settings/src/action_map.rs`, `action_and_args.rs`, `action_args.rs`, `key_chord.rs`, `command.rs`.
2. Command (nested/iterable): `Command.cpp:126/144/277/392/505/558`. Rust: no `command.rs` (Command + NestedCommands + iterable expansion).
3. Loader layering: `CascadiaSettingsSerialization.cpp:1226` `LoadAll` + `:826` `_parse` (inbox → fragments → user, with `OriginTag`) + `:314` `MergeInboxIntoUserSettings` + `Profile.cpp:239` `_FinalizeInheritance` parent graph. Rust: `app/src/settings_loader.rs:38` is single-file inbox-only; no fragment scan; no `baseLayerProfile`-as-defaults layering; no `origin` tag wired through the layer chain.
4. Generators: `PowershellCoreProfileGenerator.cpp:314` + `WslDistroGenerator.cpp:241` + `VisualStudioGenerator.cpp:30` (delegates to `VsDevCmdGenerator.cpp` / `VsDevShellGenerator.cpp`, fed by `VsSetupConfiguration.cpp`). Rust: no `crates/settings/src/profile_generators/` (or `crates/profile-generators`).
5. ApplicationState: `ApplicationState.cpp:96/149/198/264/284/310/349/431/453`. Rust: no `crates/settings/src/application_state.rs`.
6. Hot-reload: WT uses `WindowEmperor.cpp` file watcher (P16); P07 only needs the load-all trigger for tests (`ReloadDaemon`). Rust: no `crates/settings/src/reload.rs`.
7. Validation warnings: `CascadiaSettings.cpp:455-1281` + `SettingsLoadWarnings` enum (`SettingsLoadWarnings.h`). Rust: no `crates/settings/src/warnings.rs` (`SettingsLoadWarnings` enum) + `validation.rs` (9 validators).
8. Legacy migration: `MigrateReloadEnvVars` (`DeserializationTests.cpp:69`), `RoundtripLegacyToModernActions` (`SerializationTests.cpp:23`). Rust: no `legacy.rs`.
9. JSONC + round-trip unknown keys: `app/src/settings_loader.rs:107` strips comments but re-serializing via `serde_json::to_value(Profile)` drops unknown keys. Need a `JsoncValue`/`Map`-preserving deserializer or `RawValue`-based field-pass-through (WT's `JsonUtils` keeps `Json::Value` for unknown keys, emits them back on write).
10. `TerminalSettings` apply-to-control: `terminal_settings.rs:27` is frozen; P07 must ADD fields (P11 additive): `default_background_image_*`, `intense_text_style`, `cursor_height`, `padding`, `bell_style`, `tab_title`, `color_scheme_name`, `font_fallbacks`, `adjust_indistinguishable_colors`, plus `TerminalSettings::from(profile: &Profile, appearance: &Appearance, globals: &GlobalAppSettings, schemes: &HashMap<String, ColorScheme>) -> Self` resolver (lifted from WT `TerminalSettings::FromProfile`).

## Worker partition (≤4 workers, Wave-internal parallel vs sequential)
- **Wave-1A — Model/M** (parallel): extends existing structs (no loader) → adds `command.rs`, `action_args.rs`, `action_and_args.rs`, `action_map.rs`, `key_chord.rs`. Owns: `crates/settings/src/{command.rs, action_args.rs, action_and_args.rs, action_map.rs, key_chord.rs}` + `Cargo.toml` (deps only). No edit to `profile.rs`/`global_app_settings.rs` beyond additive helper `pub fn`s.
- **Wave-1B — Loader/L** (parallel with Model): `loader.rs` (calls existing struct from_json/layer_json) + `validation.rs` + `warnings.rs` + `legacy.rs` + extends `appearance.rs`/`profile.rs`/`global_app_settings.rs` only with `layer_json_fragment` / origin-tagging helpers. Owns: `crates/settings/src/{loader.rs, validation.rs, warnings.rs, legacy.rs}` + minimal edits to `appearance.rs`, `global_app_settings.rs`. Wait-on: none (uses existing struct API).
- **Wave-1C — Generators/G** (parallel): `crates/settings/src/profile_generators/{mod.rs, powershell.rs, wsl.rs, vs_setup.rs, vs_dev_cmd.rs, vs_dev_shell.rs}` (or new crate `profile-generators` — pick `settings` sub-module to keep crate count flat). Owns: that directory only. Wait-on: none.
- **Wave-1D — State/S** (parallel): `application_state.rs` + `reload.rs`. Owns: `crates/settings/src/{application_state.rs, reload.rs}`. Wait-on: none.
- **Wave-2 — Integration/I** (sequential, after 1A+1B+1C+1D): weave loader → generators → state → validation → warnings: extend `crates/settings/src/lib.rs` (module list, re-exports), `crates/settings/src/terminal_settings.rs` (ADDITIVE fields only — P01 contract frozen), `crates/settings/Cargo.toml` (notify-debouncer/winfsp/winreg deps), `crates/app/src/settings_loader.rs` (delegate to `settings::loader::load_all`).

Wave-1A/1B/1C/1D run in parallel — disjoint file ownership. Wave-2 is sequential and is the only barrier.

## Implementation steps per worker (with file:line anchors)
Wave-1A (Model): port these into `command.rs`/`action_map.rs`/`action_args.rs`/`action_and_args.rs`/`key_chord.rs`.
- `Command`. From `Command.cpp:84` ctor, `:157` `HasName`, `:277` `FromJson` (with `LayerJson` at `:392`, `ActionAndArgs::FromJson` at `:326`, nested at `:297`), `:434` `ToJson`, `:505` `ExpandCommands`, `:558` `_expandCommand` (iterable `OpenSettings`/`NewTab`/`SplitPane`), `:813` `LogSettingChanges`. Rust `Command` is a plain struct with `Arc<Command>` for nested (ADR-0011: no trait-object projection).
- `KeyChordSerialization`. Port `FromString:325` and `ToString:330`. Rust `KeyChord { ctrl, alt, shift, vkey: u32, scan_code: u32 }` plus impls for `FromString("ctrl+shift+c")` etc.
- `ActionAndArgs` / `ActionArgs`. `ActionAndArgs.cpp` + `ActionArgs.cpp:90-828`. ~60 args structs: each with `FromJson` (link to `ActionArgs.cpp` for `NewTerminalArgs`/`CopyTextArgs`/`NewTabArgs`/`MovePaneArgs`/`SwitchToTabArgs`/`ResizePaneArgs`/`MoveFocusArgs`/`SwapPaneArgs`/`AdjustFontSizeArgs`/`SendInputArgs`/`SplitPaneArgs`/`OpenSettingsArgs`/`SetFocusModeArgs`/`SetFullScreenArgs`/`SetMaximizedArgs`/`SetColorSchemeArgs`/`SetTabColorArgs`/`RenameTabArgs`/`ExecuteCommandlineArgs`/`CloseOtherTabsArgs`/`CloseTabsAfterArgs`/`CloseTabArgs`/`ScrollUpArgs`/`ScrollDownArgs`/`ScrollToMarkArgs`/`AddMarkArgs`/`MoveTabArgs`/`ToggleCommandPaletteArgs`/`SuggestionsArgs`/`FindMatchArgs`/`NewWindowArgs`/`PrevTabArgs`/`NextTabArgs`/`RenameWindowArgs`/`OpenWorkspaceArgs`/`SearchForTextArgs`/`GlobalSummonArgs`). `GenerateName` matches per-method signature.
- `ActionMap`. `ActionMap.cpp:294` `_FinalizeInheritance`, `:366` `_GetActionByID`, `:410` `AvailableActions`, `:463` `NameMap`, `:550` `_PopulateCumulativeKeyMaps`, `:590` `GlobalHotkeys`, `:599` `KeyBindings`, `:695` `AddAction`, `:840` `GetActionByKeyChord`, `:915` `GetKeyBindingForAction`, `:1041` `ExpandCommands`. `ActionMapSerialization.cpp:1-186` FromJson/LayerJson.

Wave-1B (Loader): port in `loader.rs`/`validation.rs`/`warnings.rs`/`legacy.rs` + helpers in existing structs.
- `loader.rs`: `SettingsLoader::generate_profiles()` (`CascadiaSettingsSerialization.cpp:208`), `apply_runtime_initial_settings` (`:278`), `merge_inbox_into_user_settings` (`:314`), `find_fragments_and_merge_into_user_settings` (`:329`), top-level `LoadAll` (`:1226`); per-profile `_parse:826` body — wire existing `GlobalAppSettings::from_json` (extend `global_app_settings.rs` with `FromJson`/`LayerJson`/`ToJson` mirror like `Profile_serde.rs`), `ColorScheme::from_json` (exists), `Theme::FromJson` (new `theme.rs` extension).
- `Profile.cpp:239` `_FinalizeInheritance` — port as `Profile::finalize_inheritance(&mut self, parents: &[&Profile])` in `profile_serde.rs` (additive: brand new method).
- `validation.rs`: 9 validators from `CascadiaSettings.cpp:455-1281` (schemes exist / media / env-vars / NTM regex / keybindings / color-scheme-in-commands / theme exists / regex / enabled-at-least-one-keybinding).
- `warnings.rs`: `SettingsLoadWarnings` enum (lowercase camelCase) + accumulation vector → return from `LoadAll`.
- `legacy.rs`: `MigrateReloadEnvVars` (test at `DeserializationTests.cpp:69`) + `RoundtripLegacyToModernActions` (test at `SerializationTests.cpp:23`) legacy `fontFace`/`fontSize`/`fontWeight`/`shell`→`commandline` migration.

Wave-1C (Generators): port `profile_generators/{powershell.rs, wsl.rs, vs_setup.rs, vs_dev_cmd.rs, vs_dev_shell.rs}`.
- powershell.rs: `PowershellCoreProfileGenerator.cpp:314` `GenerateProfiles` — scan `%ProgramFiles%\\PowerShell\\[7-9]*\\pwsh.exe` + `GetPreferredPowershellProfileName` (lift to local const).
- wsl.rs: `WslDistroGenerator.cpp:241` `GenerateProfiles` (registry reader via `windows` crate `HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss`).
- vs_setup.rs: `VsSetupConfiguration.cpp:1-126` — `ISetupConfiguration` COM bindings via `windows` crate; `vs_dev_cmd.rs` lifts `VsDevCmdGenerator.cpp`; `vs_dev_shell.rs` lifts `VsDevShellGenerator.cpp`.
- VisualStudio top-level `VisualStudioGenerator.cpp:30` aggregates the two.

Wave-1D (State): `application_state.rs` + `reload.rs`.
- `application_state.rs`: `ApplicationState.cpp:96` ctor (state root), `:117` dtor, `:117/149` `Flush`, `:264/284` `FromJson`/`ToJson`, `:310` `AppendPersistedWindowLayout`, `:349/362/393/431/453` workspace ops, `:490/502` shared/local content I/O. Rust: struct + `serde_json` + `std::fs` (or `tokio::fs` if P16 wants async — keep sync for P07).
- `reload.rs`: file watcher with 200ms debounce → callback registry. Use `notify` crate (background deps declared up-front in `Cargo.toml`). WT uses `WindowEmperor.cpp` watcher; P07 ships the `Watcher` struct + `register(path, cb)` API; P16 wires actual paths.

Wave-2 (Integration): wire everything.
- `lib.rs`: append modules `command, action_args, action_and_args, action_map, key_chord, loader, validation, warnings, legacy, application_state, reload, profile_generators` (+ sub-modules); re-export `CascadiaSettings, Command, ActionAndArgs, ActionMap, KeyChord, SettingsLoadWarnings`.
- `terminal_settings.rs:27`: ADDITIVE fields only (P01 freeze): append `color_scheme_name: String`, `font_fallbacks: Vec<String>`, `cursor_height: u8`, `intense_text_style`, `padding: (f32,f32,f32,f32)`, `background_image: Option<String>`, `background_image_stretch_mode`, `background_image_opacity: f32`, `background_image_alignment`, `bell_style`, `tab_title: Option<String>`, `adjust_indistinguishable_colors`. Add `TerminalSettings::from(profile, appearance, globals, schemes)` resolver (lifted from WT `TerminalSettings::FromProfile`).
- `Cargo.toml`: deps `notify = "6"`, `windows = { features = ["Win32_System_Registry", "Win32_UI_Shell", "Win32_System_Com", "Win32_Foundation"] }` (already pulled by other crates — just add features).
- `crates/app/src/settings_loader.rs`: keep `strip_jsonc_comments:107` (still used for raw input) but delegate `load_settings` to `settings::loader::load_all(&path, cx)` returning `CascadiaSettings`. Move that struct from `app` to `settings` and re-export (preserves `page.rs:32` import).

## Test gate (red tests first, cargo subset, acceptance G/W/T)
Per-worker red tests, port as Rust `#[test]` gated `cargo test -p settings -- <group>`:

Wave-1A — port from `CommandTests.cpp` + `KeyBindingsTests.cpp`:
- `layer_command` · `test_split_pane_args` · `test_autogenerated_name` · `test_layer_on_autogenerated_name` · `test_generate_commandline` (CommandTests.cpp).
- `key_chords` · `many_keys_same_action` · `layer_keybindings` · `hash_deduplication` · `unbind_keybindings` · `layer_scancode_keybindings` · `test_explicit_unbind` · `test_arbitrary_args` · `test_get_key_binding_for_action` · `keybindings_without_vkey` (KeyBindingsTests.cpp).

Wave-1B — port from `DeserializationTests.cpp` + `SerializationTests.cpp`:
- `validate_profiles_exist` (anchor `DeserializationTests.cpp:26`) · `validate_default_profile_exists` · `validate_duplicate_profiles` · `validate_many_warnings` · `layer_global_properties` · `validate_profile_ordering` · `validate_hide_profiles` · `test_reorder_with_null_guids` · `test_layering_name_only_profiles` · `test_hide_all_profiles` · `test_invalid_color_scheme_name` · `test_layer_user_defaults_before_profiles` · `test_dont_layer_guid_from_user_defaults` · `test_layer_user_defaults_on_dynamics` · `find_missing_profile` · `validate_keybindings_warnings` · `test_trailing_commas` · `migrate_reload_env_vars` (legacy.rs).
- From SerializationTests: `global_settings` · `profile` · `color_scheme` · `actions` · `cascadia_settings` · `roundtrip_reload_env_vars` · `dont_roundtrip_no_reload_env_vars` · `roundtrip_legacy_to_modern_actions` · `fixup_commandline_patching`.

Wave-1C — port from `UnitTests_TerminalCore` + new:
- Direct OOM: `TestCorrectOldDefaultShellPaths` (`ProfileTests.cpp:38`) — covers PowerShell generator naming.
- New red test: `generate_wsl_distros_from_registry_mock` (mock `HKCU/Lxss`) — generators PR.
- New red test: `vs_setup_returns_no_instances_when_setup_com_not_present`.

Wave-1D — `ApplicationStateTests.cpp`:
- `save_and_lookup_workspace` · `remove_workspace_returns_false_when_missing` · `rename_workspace_migrates_entry` · `take_workspace_removes_and_returns` · `take_workspace_returns_null_when_missing` (7 methods).

Acceptance (G/W/T, preserves brief's existing intent):
- Given the Rust `settings` crate with all Wave-1 + Wave-2 work landed, When `cargo test -p settings` runs, Then ≥ 80 of the 158 WT `UnitTests_SettingsModel` methods have Rust-port equivalents passing (target: all 9 deserialization-first + 14 keybindings + 7 application-state + 11 color-scheme + 8 command + the validation/profile/terminal-settings set).
- Given `windows/src/cascadia/TerminalSettingsModel/defaults.json` round-trips through `LoadAll → ToJson → FromJson`, When two unknown keys (`"experimental.x"` and `"futureExtension"`) are added to the user `settings.json`, Then both are preserved verbatim through the round-trip (parity with WT `JsonUtils` unknown-key pass-through).
- Given the P01 frozen `TerminalSettings` minimal struct contract (`terminal_settings.rs:27`), When P11 reads `TerminalSettings::default()`, Then no fields present in P01 are removed or renamed (additive-only contract held); new fields default to WT `DefaultSettings.h` values.
- Given a real `settings.json` fragment in `%LOCALAPPDATA%` form is loaded, When `load_all` runs on the test fixture, Then `SettingsLoadWarnings::DuplicateProfile` and `MissingDefaultProfile` are reported exactly when WT counterpart emits them on the same fixture.

## Out of scope
- Azure Cloud Shell generator (`AzureCloudShellGenerator.cpp`) — ADR-0007.
- SshHostGenerator (`SshHostGenerator.cpp`) — feature-flagged off in upstream; revisit in P19 extras.
- WinRT `IInspectable` projection / `Microsoft::Terminal::Settings::Model::*` runtime classes — ADR-0011 (plain Rust structs + `Arc`).
- `settings::editor` UI layer (`crates/settings/src/editor` skeleton) — P17.
- `app`/`control` UI wiring that consumes the resolved `TerminalSettings` (the `MakeSettingsForProfile` bridge) — P11/P13 consumers. P07 only exposes additive fields.
- `NewTabMenuEntry`/`MatchProfilesEntry`/`FolderEntry` tree (`NewTabMenuTests.cpp` deep tests) — port the entry structs minimally for `validation.rs:701`, but full new-tab-menu tree is P13.
- `MediaResourceTests` (23 tests) — icon/bell resolver is P13/P14 (only the validator hook stays in `validation.rs:654`).

## Risk assessment
- Generator Windows API surface (registry + `ISetupConfiguration`) — risk: `windows` crate feature gap or COM threading. Mitigate: gate generators behind `cfg(windows)` + allow `--no-default-features` build that stubs them out, so `cargo test --no-default-features` still runs model/loader/red tests.
- Unknown-key round-trip — risk: `serde` derive drops unknown keys unless we wrap with `serde_json::Value::Object` pass-through. Mitigation: store a `RawValue`-backed `unknown: serde_json::Map<String, Value>` per model (extends `Profile.serde` and `GlobalAppSettings` to capture non-tagged keys on deserialize, emit them on `to_json`).
- Profile inheritance graph — risk: `Arc<Profile>` parent chain could form cycles if a fragment's `updates` points at itself (test exists in WT `LoadFragmentsWithMultipleUpdates`). Mitigation: detect cycles in `Profile::finalize_inheritance` with a visited set, error instead of looping.
- JSONC parser — risk: existing `strip_jsonc_comments:107` does not handle escaped `/*` inside `//` line comments (rare in real settings). Mitigation: keep current parser for P07; add a `// P14 TODO` marker for the trailing-comma edge case `}]` → already handled.
- Hot-reload deps — risk: `notify` v6 changes callback signature; thread safety in GPUI context. Mitigation: keep `reload.rs` minimal (callback runs on cleanup thread; P16 owns GPUI dispatch).
- File ownership across Wave-1 — Model and Loader both add helpers to `appearance.rs:17` / `global_app_settings.rs:25`. Mitigation: Loader only adds `layer_json_fragment` methods under `impl Appearance` and `impl GlobalAppSettings` (model ports `FromJson`/`LayerJson`/`ToJson` in a sibling `*_serde.rs`), preserving disjoint edit regions inside the same file.
