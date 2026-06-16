# Grading criteria — VIGIL

> Official rubric, reformatted as tables. Each Sprint 1 criterion is worth **1 point**.
> The wording is the generic VIGIL spec (servers / channels / messages); OpsWarden
> reinterprets it as **team / incident / timeline entry** — see `consignes_VIGIL.md`.

## Sprint 1

| ID                    | Criterion                                                                                                       | Status |
| --------------------- | --------------------------------------------------------------------------------------------------------------- | ------ |
| `specs_server`        | The server uses NodeJS or Rust and allows simultaneous connections                                              | ✅     |
| `specs_client`        | The client uses ReactJS or NextJS and is connected to the server                                                | ⏳     |
| `user_list`           | Users can see who joined the server                                                                             | ⏳     |
| `chan_list`           | Users can list all channels inside a server                                                                     | ⏳     |
| `server_create`       | Users can create a server                                                                                       | ⏳     |
| `server_delete`       | Users can delete a server                                                                                       | ⏳     |
| `server_join`         | Users can join a server                                                                                         | ⏳     |
| `server_multiple`     | Users can join multiple servers simultaneously                                                                  | ⏳     |
| `server_quit`         | Users can leave a server                                                                                        | ⏳     |
| `chan_create`         | Users can create a channel inside a server                                                                      | ⏳     |
| `chan_delete`         | Users can delete a channel inside a server                                                                      | ⏳     |
| `chan_message`        | Users can send a message to all users in a channel using WebSocket                                              | ⏳     |
| `status_online`       | Users can see who is online on the server                                                                       | ⏳     |
| `status_typing`       | Users can see who is typing inside a channel                                                                    | ⏳     |
| `user_management`     | Different roles are available allowing different permissions inside a server                                    | ⏳     |
| `persistency`         | Servers, channels and messages are persistently preserved                                                       | ✅     |
| `functional-delivery` | The delivery is functional, most of the previous achievements are obtained                                      | ⏳     |
| `ui_servers`          | The servers management interface is clear and intuitive                                                         | ❌     |
| `ui_chat`             | The chat interface inside a channel is clear and intuitive                                                      | ❌     |
| `ui_design`           | The interface design is well elaborated and advanced                                                            | ❌     |
| `uiux_quality`        | The delivery offers a high-quality, polished UX and UI                                                          | ❌     |
| `versioning_basics`   | Versioning tool with a proper workflow: branching strategy, regular commits, descriptive messages, `.gitignore` | ✅     |
| `coding_style`        | The code respects a common coding style                                                                         | ⏳     |
| `tests_unit`          | At least 70% of the source code is tested                                                                       | ⏳     |
| `tests_automation`    | The tests are easily runnable                                                                                   | ✅     |
| `tests_coverage`      | Most branches are tested, not only the main flow                                                                | ⏳     |
| `documentation`       | A README is delivered and the project is documented for newcomers                                               | ⏳     |
| `presentation`        | The project is presented professionally using a relevant support (slides and/or demo)                           | ❌     |
| `extra_small`         | At least 1 feature not listed in the "features" section                                                         | ❌     |
| `extra_medium`        | At least 3 features not listed in the "features" section                                                        | ❌     |
| `extra_large`         | More than 4 features not listed in the "features" section                                                       | ❌     |

**Total possible: 31 points.**

## Sprint 2

| ID                      | Criterion                                                                                              | Status |
| ----------------------- | ------------------------------------------------------------------------------------------------------ | ------ |
| `milestone_1`           | The first milestone is achieved and complete                                                           | ⏳     |
| `milestone_2`           | The second milestone is achieved and complete                                                          | ❌     |
| `milestone_3`           | The third milestone is achieved and complete                                                           | ❌     |
| `web_server`            | The server uses NodeJS or Rust and allows simultaneous connections                                     | ✅     |
| `web_client`            | The client uses ReactJS or NextJS and is connected to the server                                       | ⏳     |
| `web_core_features`     | ALL the core features (kick, temp/permanent bans, message editing) are complete and functional         | ❌     |
| `web_multilingual`      | The web app interface can switch between at least two languages                                        | ❌     |
| `web_api_integration`   | An external GIF API is properly integrated                                                             | ❌     |
| `web_pm`                | Users can send private messages between each other                                                     | ❌     |
| `web_reactions`         | Users can react to others' messages with emojis                                                        | ❌     |
| `desktop_app`           | A runnable and functional desktop app is delivered                                                     | ❌     |
| `desktop_specs`         | The desktop application uses Tauri or ElectronJS and is connected to the server                        | ❌     |
| `desktop_multilingual`  | Desktop app is translated (at least 2 languages)                                                       | ❌     |
| `desktop_notifications` | Desktop app contains a notifications system                                                            | ❌     |
| `tests_unit`            | At least 70% of the source code is tested                                                              | ⏳     |
| `tests_sequence`        | A sequence of tests is delivered and easily runnable                                                   | ⏳     |
| `tests_automation`      | A test sequence is automatically launched through the CI pipeline                                      | ⏳     |
| `tests_coverage`        | An evaluation of the proportion of source code tested is delivered                                     | ⏳     |
| `repo_versioning`       | Version control workflow: branching strategy, regular commits, descriptive messages, `.gitignore`      | ✅     |
| `repo_secrets`          | Secrets (tokens, passwords, keys...) are not committed in clear-text nor visible to non-granted people | ⏳     |
| `repo_cicd`             | The project automatically runs tests and creates a build when a tag is created                         | ⏳     |
| `repo_doc`              | A README is delivered and the project is documented for newcomers                                      | ⏳     |
| `code_style`            | Code follows the language's best practices and consistent coding standards                             | ⏳     |
| `code_maintainability`  | The code is easily maintainable (readable names, atomic functions, clear structure, clean syntax)      | ⏳     |
| `proj_pres`             | The project is presented professionally using a relevant support (slides and/or demo)                  | ❌     |
| `proj_review`           | One feature is reviewed during the presentation                                                        | ❌     |
| `proj_answers`          | Students can answer questions asked during the presentation                                            | ❌     |
| `proj_orga`             | Students can show proof of their working organization (task board, commit logs, etc.)                  | ⏳     |
| `extra_small`           | At least 1 feature not listed in the "Project Objectives" section                                      | ❌     |
| `extra_medium`          | At least 3 extra features not listed in the project                                                    | ❌     |
| `extra_large`           | More than 5 extra features not listed in the project                                                   | ❌     |
