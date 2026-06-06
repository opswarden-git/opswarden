# OpsWarden — Roadmap d'exécution (VIGIL rattrapage)

> **Projet** : OpsWarden — Incident Management Platform (sujet VIGIL).
> **Contexte** : solo, rattrapage T-DEV-600, niveau visé senior-grade + portfolio public.
> **Fenêtre** : 5 juin 2026 → veille de rentrée septembre ≈ **12 semaines**.
> **Archi verrouillée** : monorepo modulaire (cargo workspace + npm workspaces), hexagonal Rust/Axum, agent RAG = service unique extrait derrière un port, couche cloud/IaC en repo `opswarden-ops` séparé **hors chemin de notation**.

---

## 0. Principe directeur (à relire chaque lundi)

**Sécuriser le « pass » le plus tôt possible, empiler la valeur ensuite.**

Le sujet pénalise une démo qui crashe plus qu'une feature manquante. La conséquence stratégique est non négociable :

- **Fin Semaine 9, le projet doit être techniquement validable** (les 3 phases *core* passent, `docker compose up` tourne, CI verte).
- Les 3 semaines restantes sont du **grade-boosting + portfolio** (AI SRE, releases, modération, i18n, couverture, vitrine cloud, keynote).
- Si on prend du retard : on coupe dans l'*extended*, **jamais** dans le *core*. Le core est intouchable.

Règle d'or scope : **le produit (monorepo) ne dépend jamais de la vitrine (cloud).** Le jury exécute `docker compose up`, pas `kubectl apply`.

---

## 1. Stratégie de branching

**Modèle : trunk-based + branches courtes + PR vers `main`** (même en solo). Pourquoi : `main` reste toujours déployable et verte, chaque PR déclenche la CI = preuve de discipline pro, historique propre pour le code walkthrough du jury.

### Règles
- `main` **protégée** : merge interdit si la CI échoue. (Branch protection rule GitHub.)
- Branches courtes (< 2-3 jours de vie), une par unité de travail :
  - `feat/<scope>` — nouvelle feature (`feat/auth-jwt`, `feat/incident-lifecycle`)
  - `fix/<scope>` — correction
  - `chore/<scope>` — outillage, deps, CI
  - `docs/<scope>` — documentation
  - `test/<scope>` — ajout/renfort de tests
- **Commits conventionnels** (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`) → alimente le changelog auto du workflow release.
- **Squash merge** vers `main` (1 PR = 1 commit propre sur main).
- **Tags `vX.Y.Z`** déclenchent le pipeline release (build binaire desktop + image Docker), comme exigé par le sujet.

### Convention de commits

Format **Conventional Commits** : `type(scope): sujet à l'impératif`.

- `type` ∈ `feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`, `ci`, `build`.
- `scope` = zone touchée (`auth`, `incidents`, `ws`, `compose`, `ci`...). Optionnel mais recommandé.
- Sujet court (≤ 72 car.), impératif, sans point final (`add JWT logout invalidation`).
- **Corps** (optionnel) : le *pourquoi*, pas le *quoi* — le diff dit déjà le quoi.
- **Footer** : `BREAKING CHANGE:` si rupture d'API ; `Refs #123` pour lier une issue.
- Commits **atomiques** : 1 commit = 1 changement cohérent qui compile et passe les tests.
- Commits assistés par IA : ajouter `Co-Authored-By:` en footer (traçabilité honnête).

**Lien avec la release** : chaque PR est *squashée* → le message de squash (conventionnel) devient l'unique commit sur `main` et alimente le changelog auto du workflow release. Soigner le titre de PR = soigner le changelog.

```
feat(auth): add JWT logout with server-side token invalidation
fix(ws): reconnect client after broadcaster restart
test(incidents): cover escalation error paths
chore(ci): wire clippy and prettier on every branch push
```

### Cadence de tags (jalons)
| Tag | Quand | Signifie |
|---|---|---|
| `v0.1.0` | Fin S2 (Sem 5) | Phase 1 core validée |
| `v0.2.0` | Fin S3 (Sem 7) | Phase 2 core validée |
| `v1.0.0` | Fin S4 (Sem 9) | Phase 3 core validée → **projet passe** |
| `v1.1.0` | Fin S5 (Sem 11) | Extended (AI SRE, releases, modération, i18n) |
| `v1.2.0` | S6 (Sem 12) | Polish + vitrine cloud + freeze démo |

---

## 2. Cadence agile (6 sprints solo)

Sprints de 2 semaines (sauf S0 = 1 semaine de rails et S6 = 1 semaine de freeze). « Scrum solo » = léger : un **objectif de sprint**, une **definition of done**, un **incrément démo-able** à la fin de chaque sprint.

```
S0  | Sem 1     | Foundations & rails (repo, CI verte, compose v0)
S1  | Sem 2-3   | Phase 1 core A : auth + teams + RBAC
S2  | Sem 4-5   | Phase 1 core B : incidents + timeline + WS   → PHASE 1 ✅  (v0.1.0)
S3  | Sem 6-7   | Phase 2 core : rule engine + about.json + vault → PHASE 2 ✅ (v0.2.0)
S4  | Sem 8-9   | Phase 3 core : desktop Tauri + notifs + compose → PHASE 3 ✅ (v1.0.0)
S5  | Sem 10-11 | Extended : AI SRE/RAG + releases + modération + i18n + coverage (v1.1.0)
S6  | Sem 12    | Freeze : hardening + vitrine cloud + démo + keynote (v1.2.0)
```

**Lecture clé** : le projet est *passable* dès la fin de la Semaine 9. Tout ce qui suit est du bonus.

---

## 3. Definition of Done — globale (s'applique à CHAQUE PR)

Une PR ne merge sur `main` que si **tout** est vrai :
- [ ] CI verte : `cargo clippy` (0 warning), `cargo fmt --check`, ESLint, `prettier --check`, tests unitaires.
- [ ] Tests ajoutés pour le nouveau comportement (happy path **+ au moins un chemin d'erreur**).
- [ ] Logique métier dans `domain`/`app`, **jamais** dans les handlers ni les clients.
- [ ] Codes d'erreur HTTP corrects sur routes protégées (401/403/404).
- [ ] Doc impactée mise à jour (README / WEBSOCKET_SPEC / etc.).
- [ ] Commit conventionnel, atomique, message explicite.
- [ ] Preuve par sortie shell brute jointe à la PR (build + test qui passent).

### Rampe de couverture — le 70 % ne se rattrape pas en S5

70 % de lignes **+ branches** ne se bricolent pas en fin de course : on teste **à chaque PR** (DoD ci-dessus) et on vise une cible par phase. L'archi hexagonale rend ça bon marché : `domain` + `app` se testent **sans DB ni HTTP** (ports mockés), donc la couverture vient des use-cases, pas de tests d'intégration coûteux.

| Phase | Cible lignes | Où on gagne la couverture |
|---|---|---|
| S0 | harness en place (3 tests verts) | `health`, `about.json` (token SHA-256), `sha256_hex` |
| S1 | ≥ 40 % | use-cases auth/teams/RBAC + invariants domaine (ports mockés) ; 403 testés |
| S2 | ≥ 55 % | machines à états incident, présence, broadcaster (`EventBus` mocké) |
| S3 | ≥ 65 % | hook engine (trigger/filtre/reaction), HMAC, vault (chiffré ↔ déchiffré) |
| S4 | ≥ 70 % (et on tient) | parcours bout-en-bout, intégration handlers |
| S5 | ≥ 70 % verrouillé + branches | rapport `cargo tarpaulin` en **artifact CI**, chemins d'erreur explicites |

Outils : `cargo tarpaulin` (serveur), couverture du test runner côté client. Le rapport devient un **artifact CI sur merge `main` dès la Phase 2** (pas en S5). `test/<scope>` est un type de branche à part entière : renforcer les tests est un travail légitime, pas un reliquat.

---

## 4. Détail des sprints

### S0 — Foundations & rails (Sem 1)
**Objectif** : un squelette qui tourne et une CI verte avant la première ligne de métier.

Scope :
- Scaffold monorepo : `cargo workspace` (`server`, futur `investigation`) + `npm workspaces` (`client-web`, `client-desktop`).
- Squelette hexagonal serveur : `domain/ ports/ adapters/ handlers/ app/` + endpoint `/health`.
- `docker-compose.yml` v0 : `server` (8080) + `db` (Postgres) qui démarrent *healthy*.
- CI GitHub Actions : sur push → clippy + fmt + ESLint + prettier + tests ; jobs en parallèle. Badge dans le README.
- Branch protection sur `main`, commits conventionnels, template de PR.
- `/about.json` minimal avec **token placeholder** (champ config 1 ligne, vraie valeur au kickoff).
- **Admin** : déclarer les exemptions T-DEV-600 (CI/CD, i18n) avec preuve si validées ; sinon les planifier comme core.

Note critique :
- La valeur exacte du token `/about.json` n'est pas un choix produit. Elle doit être injectée par config au kickoff, puis hashée en SHA-256. On documente un placeholder, on n'éparpille jamais une valeur hardcodée dans le repo.

Branches : `chore/repo-scaffold`, `chore/ci-pipeline`, `chore/docker-compose-v0`, `feat/health-about`.

DoD S0 :
- [ ] `docker compose up` → serveur + db healthy.
- [ ] Badge CI vert sur le README.
- [ ] `GET /health` et `GET /about.json` répondent.

---

### S1 — Phase 1 core A : Auth + Teams + RBAC (Sem 2-3)
**Objectif** : un utilisateur s'inscrit, crée une team, le système de rôles est appliqué côté serveur.

Scope :
- Auth email/password (hash argon2/bcrypt), JWT, `GET /me`, logout avec **invalidation de token** (blacklist/rotation persistée).
- Teams : créer, rejoindre par **code d'invitation**, 3 rôles (Observer/Responder/Manager), **transfert du rôle Manager**, contrainte « 1 seul Manager, ne peut pas partir sans transférer ».
- Permissions appliquées dans `app` (use-cases), pas dans les handlers. Codes 401/403 propres.
- Persistance Postgres via SQLx, migrations versionnées.

Branches : `feat/auth-jwt`, `feat/me-logout`, `feat/teams-crud`, `feat/rbac-roles`, `feat/manager-transfer`.

DoD S1 :
- [x] Parcours signup → login → `/me` → logout (token mort après). *(✅ API Server prête)*
- [x] Un Observer se voit refuser une action Responder (403 testé).
- [x] Transfert Manager fonctionnel + invariant « 1 Manager » testé.

---

### S2 — Phase 1 core B : Incidents + Timeline + WebSockets (Sem 4-5) → **Phase 1 validée**
**Objectif** : le cœur temps réel collaboratif.

Scope :
- Incidents : cycle `open → acknowledged → escalated → resolved`, sévérité `low/medium/high/critical`.
- Assignation d'un Responder par le Manager (notif WS immédiate).
- **Timeline temps réel** (entrées horodatées, visibles par tous les connectés).
- WS broadcaster (adapter) + les 5 events core : `incident_state_changed`, `incident_escalated`, `incident_assigned`, `timeline_entry_added`, `presence_update`.
- **Présence** (qui regarde quoi) + **reconnexion auto côté client** (obligatoire dès cette phase).

Branches : `feat/incident-lifecycle`, `feat/incident-severity`, `feat/timeline`, `feat/ws-broadcaster`, `feat/presence`, `feat/ws-reconnect`.

DoD S2 :
- [ ] Deux navigateurs : une action sur l'un se reflète en < 1s sur l'autre via WS. *(✅ Serveur WS prêt, ⏳ Front)*
- [ ] Coupure réseau → reconnexion auto, état resynchronisé. *(⏳ Front)*
- [x] Tous les events core émis et documentés (début de `WEBSOCKET_SPEC.md`).
- [ ] **Tag `v0.1.0`** — Phase 1 core complète.

---

### S3 — Phase 2 core : Rule engine + about.json + Vault (Sem 6-7) → **Phase 2 validée**
**Objectif** : l'automatisation Action→REAction, end-to-end, robuste.

Scope :
- Webhook receiver `POST /webhooks/{service}` + **validation HMAC**.
- Hook engine : évaluation de règle (trigger + filtres → reaction).
- **1 service Action** : GitHub (`workflow_run: failure`). **1 REAction** au-delà de VIGIL : Slack (ou HTTP/Email). VIGIL REAction : `create_incident`.
- **1 règle complète démontrable** : CI GitHub échoue → Incident `high` créé automatiquement.
- `/about.json` : catalogue de services **dynamique** + token SHA-256 (valeur kickoff).
- **Vault** : stockage chiffré côté serveur des tokens OAuth2/perso (AES-GCM, clé via env/secret ; jamais en clair).
- WS : `rule_triggered`, `rule_failed`.

Branches : `feat/webhook-receiver-hmac`, `feat/hook-engine`, `feat/action-github`, `feat/reaction-vigil-slack`, `feat/about-json-dynamic`, `feat/token-vault`, `feat/ws-rule-events`.

DoD S3 :
- [ ] Démo live : push qui casse la CI GitHub → Incident apparaît tout seul dans OpsWarden.
- [ ] `/about.json` reflète le catalogue réel (rien de hardcodé côté client).
- [ ] Token en base illisible en clair (preuve : `SELECT` qui montre du chiffré).
- [ ] **Tag `v0.2.0`** — Phase 2 core complète.

---

### S4 — Phase 3 core : Desktop + Notifs + Compose final (Sem 8-9) → **Phase 3 validée → PROJET PASSE**
**Objectif** : l'app desktop native + le contrat docker-compose complet.

Scope :
- App **Tauri** installable (cible **Linux / AppImage**, documentée dans le README), réutilisant le front Next.
- **Notifications OS natives** sur les 3 triggers : assignation, sévérité `critical`, Release bloquée. + tray icon (reste actif fenêtre fermée).
- `docker-compose.yml` **final** : `server` (8080), `client_web` (8081), `client_desktop` (build binaire, **volume partagé** avec `client_web`), `db`. `client_web` dépend de `server` **ET** `client_desktop`. Binaire exposé : `GET http://localhost:8081/client.AppImage`.
- `README.md` complet : archi + diagramme, justification des 3 choix libres, install, doc REST complète, schéma BDD commenté, section « où vit quoi » (handlers/services/domain/persistence/WS).

Branches : `feat/tauri-shell`, `feat/native-notifications`, `feat/tray-icon`, `chore/docker-compose-final`, `docs/readme-complete`.

DoD S4 :
- [ ] AppImage installable, toutes les features VIGIL utilisables depuis le desktop.
- [ ] Les 3 notifs natives se déclenchent (démo).
- [ ] `docker compose up` à froid → web télécharge le binaire desktop sur :8081.
- [ ] README permet à un inconnu de lancer le projet sans toi.
- [ ] **Tag `v1.0.0`** — les 3 phases core validées. **Le projet est notable.**

---

### S5 — Extended : AI SRE + Releases + Modération + i18n + Coverage (Sem 10-11)
**Objectif** : empiler la valeur qui démarque (et fait gagner le master).

Scope (par ordre de priorité, on s'arrête où le temps s'arrête) :
1. **Agent RAG / AI SRE** (ton différenciateur) : service unique `investigation` derrière un port `InvestigationEngine`. Chaîne : Incident créé → l'agent lit logs d'erreur + diff du commit + incidents passés similaires → poste une hypothèse de cause racine + runbook suggéré dans la timeline. Endpoints `@ask` / `@search`.
2. **Releases** : cycle complet + **blocage auto** par Incident lié (`in_progress → blocked`), `release_step_validated`, `release_state_changed`.
3. **Modération** : kick / ban temporaire / ban permanent (l'historique reste attribué). Events `member_kicked`, `member_banned`.
4. **i18n FR/EN** (si non exempté) : labels, états, sévérités ; langue persistée serveur.
5. **Couverture ≥ 70% lignes** + branches au-delà du happy path ; rapport en artifact CI (`cargo tarpaulin`).
6. Si rab : timeline editing, réactions, messages privés, OAuth2 GitHub.

Branches : `feat/investigation-agent`, `feat/releases-lifecycle`, `feat/release-auto-block`, `feat/moderation`, `feat/i18n`, `test/coverage-70`.

DoD S5 :
- [ ] Démo AI SRE : incident auto-créé → l'agent poste une analyse plausible en < 30s.
- [ ] Une Release passe `blocked` quand on lie un incident actif (démo).
- [ ] Couverture ≥ 70% visible dans l'artifact CI.
- [ ] **Tag `v1.1.0`**.

---

### S6 — Freeze, vitrine cloud & keynote (Sem 12)
**Objectif** : zéro régression, démo carrée, portfolio prêt. **Aucune feature nouvelle sur le produit.**

Scope :
- **Hardening** : chasse aux bugs, edge cases, messages d'erreur, dark patterns (confirmations destructives nommant la ressource).
- Docs restantes : `WEBSOCKET_SPEC.md`, `HOWTOCONTRIBUTE.md` (comment ajouter service/Action/REAction/event), `UI_GUIDELINES.md` (palette, mapping états→visuel, composants, dark patterns, **≥ 2 screenshots annotés**).
- **Vitrine cloud** (repo `opswarden-ops`, hors notation) : Vercel (web) + droplet/DOKS via terraform + Traefik + OTel/Grafana/Loki si le temps le permet. **Toujours optionnel.**
- **Démo de secours enregistrée** (vidéo) sur environnement stable isolé.
- **Keynote** répétée (voir §6).

DoD S6 :
- [ ] Répétition de démo complète sans crash, deux fois d'affilée.
- [ ] Backup vidéo prêt.
- [ ] Les 4 docs complètes et à jour.
- [ ] **Tag `v1.2.0`**.

---

## 5. Mapping WebSocket events → sprints

| Event | Sprint | Scope |
|---|---|---|
| `incident_state_changed`, `incident_escalated`, `incident_assigned`, `timeline_entry_added`, `presence_update` | S2 | core |
| `rule_triggered`, `rule_failed` | S3 | core |
| `release_step_validated`, `release_state_changed` | S5 | extended |
| `member_kicked`, `member_banned` | S5 | extended |
| `timeline_entry_edited`, `private_message_received`, `reaction_added`, `reaction_removed` | S5 (si rab) | extended |

Note : `private_message_received` n'est envoyé qu'à l'émetteur + destinataire (pas broadcast). `member_banned.until = null` pour un ban permanent.

---

## 5b. Sync notation (`grading-criteria.md`) ↔ sprints

> `grading-criteria.md` est la grille **héritée** des modules T-JSF-600 / T-DEV-600, avec des libellés **RTC** (*server*, *channel*, *typing*). VIGIL est noté sur **son propre** périmètre (core/extended du sujet, détaillé en §4) **plus** les achievements transverses hérités ci-dessous. Cette table mappe ces transverses au sprint où on les gagne, et retraduit les libellés chat.

| Achievement (ID grading) | Sens VIGIL | Gagné en |
|---|---|---|
| `specs_server` / `specs_client` (`web_server`/`web_client`) | serveur Rust multi-connexion + client Next connecté | S0–S1 |
| `versioning_basics` / `repo_versioning` | trunk-based, commits conventionnels, `.gitignore` | **S0 (dès l'init)** |
| `coding_style` / `code_style` | clippy + fmt + ESLint + prettier au vert | **S0**, puis chaque PR |
| `tests_automation` / `tests_sequence` | tests lancés par la CI | **S0** |
| `tests_unit` | ≥ 70 % lignes (voir rampe §3) | S1 → S5 |
| `tests_coverage` | rapport coverage en artifact + branches hors happy-path | S3 (artifact) → S5 (≥ 70 %) |
| `repo_cicd` | lint/test push, build+coverage main, artifacts tag | S0 (vert) → S3 (complet) |
| `repo_secrets` | tokens chiffrés, rien en clair | **S3 (vault)** |
| `persistency` | Postgres, tout persisté | S1 |
| `user_management` | RBAC 3 rôles (≠ « server roles » du chat) | S1 |
| `code_maintainability` / `repo_doc` / `documentation` | hexagonal lisible + README « où vit quoi » | S4 (+ continu) |
| `ui_servers` / `ui_chat` / `ui_design` / `uiux_quality` | UI teams / incidents / timeline (≠ chat) | S2 → S4 |
| `desktop_app` / `desktop_specs` / `desktop_notifications` | Tauri + notifs OS | S4 |
| `web_multilingual` / `desktop_multilingual` | i18n FR/EN | S5 |
| `web_core_features` / `web_pm` / `web_reactions` | modération + édition / messages privés / réactions | S5 |
| `presentation` / `proj_pres` / `proj_review` / `proj_answers` / `proj_orga` | keynote + board + démo | S6 |
| `extra_small` / `extra_medium` / `extra_large` | features hors sujet → **AI SRE, releases, GitLab…** | S5 |

> Les libellés purement chat (`chan_create`, `chan_delete`, `status_typing`…) n'ont **pas** d'équivalent direct : OpsWarden n'a pas de channels. Leur valeur conceptuelle (créer/lister des ressources, présence) est couverte par les features VIGIL en §4 (teams, incidents, timeline, présence).
>
> **Lecture clé** : la discipline notée (versioning, lint, CI, tests) se gagne **dès S0**, pas à la fin — d'où la rampe de couverture du §3.

---

## 6. Keynote — mapping sur la structure imposée (préparé dès S5)

1. **Introduction** : le problème (la « coordination tax », MTTR) — ouvrir sur la recomposition du marché (Opsgenie EOL avril 2027).
2. **Archi technique** : monolithe modulaire hexagonal Rust + justification des 3 choix libres + agent RAG extrait.
3. **Méthodologie** : trunk-based, 6 sprints, CI verte en permanence, « pass sécurisé S9 puis valeur empilée ».
4. **Démo live** : la chaîne Action→REAction (CI cassée → incident) + l'AI SRE qui investigue + notif desktop native. Environnement stable, backup vidéo prêt.
5. **Code walkthrough** : choisir **le rule engine** ou **l'AI SRE** (la pièce la plus impressionnante) — montrer la séparation domain/ports/adapters.
6. **Q&A** : préparer les questions « pourquoi pas microservices ? » (réponse : modular monolith = consensus 2026 solo, découpable plus tard) et « pourquoi Rust/Tauri/Postgres ? ».

### Angle de pitch produit

- Le sujet couvre déjà les briques "collaboration + lifecycle + automatisation" d'une vraie Incident Management Platform.
- Les briques non demandées mais très valorisantes sont : on-call, status page, postmortem, analytics MTTR, AI SRE.
- Parmi elles, **AI SRE** est le meilleur levier de différenciation pour OpsWarden : plus impressionnant qu'une accumulation de features périphériques, et directement cohérent avec le moteur Action→REAction.

---

## 7. Risk register (anti-patterns à éviter)

| Risque | Signal | Parade |
|---|---|---|
| Démo qui crashe | Dépendance au cloud/k8s le jour J | Démo sur `docker compose` local + backup vidéo |
| Scope creep cloud trop tôt | On code des YAML k8s en juillet | `opswarden-ops` gelé jusqu'à S6 |
| Retard sur le core | Phase 1 pas finie fin S5 | Couper dans l'extended, jamais le core |
| Couverture sous 70% en fin de course | Tests repoussés | Tester à chaque PR (DoD), pas en bloc en S5 |
| CI cassée en semaine 10 | « je la réparerai plus tard » | CI verte dès S0, jamais merge sur rouge |
| Logique métier dans les clients | Handlers/clients qui grossissent | Revue DoD systématique |

---

## 8. Repo `opswarden-ops` (vitrine, hors notation)

Séparé du monorepo produit. Contenu (best-effort, S6) :
- `terraform/` — provisioning DOKS (DigitalOcean, student pack).
- `k8s/` — manifests (Traefik, Postgres, Redis, services, ingress, cAdvisor).
- `bootstrap/` — exercices Minikube local.
- CI « local → cloud » séparée.
- `docs/architecture.png`.

**Contrat** : ce repo ne doit jamais être un prérequis pour faire tourner OpsWarden. C'est la cerise portfolio, pas le gâteau.

---

*Roadmap v1 — à réviser en fin de chaque sprint (rétro solo de 30 min : qu'est-ce qui a marché, qu'est-ce qu'on coupe).*
