fn localize_capability(
    service: &str,
    kind: &str,
    locale: &str,
    fallback: &str,
    label: bool,
) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match (service, kind, label) {
        (GITLAB_SERVICE, "ci_failed", true) => "Échec d’une pipeline CI",
        (GITLAB_SERVICE, "ci_failed", false) => {
            "Une pipeline GitLab CI/CD s’est terminée avec un statut en échec"
        }
        (GITHUB_SERVICE, "ci_failed", true) => "Échec d’un workflow CI",
        (GITHUB_SERVICE, "ci_failed", false) => {
            "Un workflow GitHub Actions s’est terminé avec un résultat en échec"
        }
        (GITLAB_SERVICE, "ci_succeeded", true) => "Succès d’une pipeline CI",
        (GITLAB_SERVICE, "ci_succeeded", false) => {
            "Une pipeline GitLab CI/CD s’est terminée avec succès"
        }
        (GITHUB_SERVICE, "ci_succeeded", true) => "Succès d’un workflow CI",
        (GITHUB_SERVICE, "ci_succeeded", false) => {
            "Un workflow GitHub Actions s’est terminé avec succès"
        }
        (GITHUB_SERVICE | GITLAB_SERVICE, "tag_pushed", true) => "Nouveau tag poussé",
        (GITLAB_SERVICE, "tag_pushed", false) => {
            "Un nouveau tag Git a été poussé dans le projet GitLab"
        }
        (GITHUB_SERVICE, "tag_pushed", false) => "Un nouveau tag Git a été poussé dans le dépôt",
        (GITHUB_SERVICE, "pr_merged", true) => "Pull request fusionnée",
        (GITHUB_SERVICE, "pr_merged", false) => "Une pull request a été fusionnée dans le dépôt",
        (OPSWARDEN_SERVICE, "release_created", true) => "Release créée",
        (OPSWARDEN_SERVICE, "release_created", false) => "Une Release a été créée dans l’équipe",
        (GENERIC_SERVICE, "generic_event", true) => "Événement JSON générique",
        (GENERIC_SERVICE, "generic_event", false) => {
            "Un webhook JSON borné et indépendant du fournisseur a été reçu"
        }
        (ALERTMANAGER_SERVICE, "alert_firing", true) => "Alerte active",
        (ALERTMANAGER_SERVICE, "alert_firing", false) => {
            "Une alerte Alertmanager est devenue active"
        }
        (ALERTMANAGER_SERVICE, "alert_resolved", true) => "Alerte résolue",
        (ALERTMANAGER_SERVICE, "alert_resolved", false) => "Une alerte Alertmanager a été résolue",
        (TIMER_SERVICE, "daily_at", true) => "Tous les jours à une heure locale",
        (TIMER_SERVICE, "daily_at", false) => {
            "Exécuter une fois par jour calendaire local à l’heure configurée"
        }
        (TIMER_SERVICE, "every_minutes", true) => "Toutes les N minutes",
        (TIMER_SERVICE, "every_minutes", false) => {
            "Exécuter selon un intervalle borné en minutes écoulées"
        }
        (OPSWARDEN_SERVICE, "create_incident", true) => "Créer un incident",
        (OPSWARDEN_SERVICE, "create_incident", false) => {
            "Ouvrir un incident dans l’équipe propriétaire de la règle"
        }
        (OPSWARDEN_SERVICE, "validate_release_step", true) => "Valider une étape de Release",
        (OPSWARDEN_SERVICE, "validate_release_step", false) => {
            "Valider la prochaine étape séquentielle d’une Release"
        }
        (OPSWARDEN_SERVICE, "block_release", true) => "Bloquer une Release",
        (OPSWARDEN_SERVICE, "block_release", false) => {
            "Créer et lier un Incident actif à une Release en cours"
        }
        (OPSWARDEN_SERVICE, "escalate_incident", true) => "Escalader un Incident",
        (OPSWARDEN_SERVICE, "escalate_incident", false) => {
            "Escalader un Incident acquitté en respectant son cycle de vie"
        }
        (HTTP_SERVICE, "http_notify", true) => "Envoyer une notification HTTP",
        (HTTP_SERVICE, "http_notify", false) => {
            "Envoyer une notification via une connexion HTTP configurée"
        }
        (EMAIL_SERVICE, "email_notify", true) => "Envoyer un e-mail",
        (EMAIL_SERVICE, "email_notify", false) => "Envoyer un e-mail à une adresse configurée",
        _ => fallback,
    }
    .to_string()
}

fn localize_connection(service: &str, locale: &str, fallback: &str) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match service {
        GITHUB_SERVICE => {
            "Vérifier les webhooks entrants et autoriser facultativement l’accès à l’API GitHub"
        }
        GITLAB_SERVICE => "Vérifier les webhooks GitLab entrants avec leur jeton secret",
        GENERIC_SERVICE => "Recevoir des webhooks JSON bornés authentifiés par un jeton partagé",
        ALERTMANAGER_SERVICE => {
            "Recevoir les groupes Alertmanager authentifiés par un jeton Bearer"
        }
        HTTP_SERVICE => "Envoyer des notifications bornées vers un endpoint HTTPS public",
        EMAIL_SERVICE => "Configurer les identifiants SMTP pour l’envoi d’e-mails",
        _ => fallback,
    }
    .to_string()
}

fn localize_oauth(service: &str, locale: &str, fallback: &str, label: bool) -> String {
    if locale == "fr" && service == GITHUB_SERVICE {
        if label {
            "Autoriser avec GitHub"
        } else {
            "Les jetons d’accès et de rafraîchissement restent chiffrés sur le serveur"
        }
    } else {
        fallback
    }
    .to_string()
}

fn localize_field(
    service: &str,
    owner: &str,
    name: &str,
    locale: &str,
    fallback: &str,
    label: bool,
) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    let qualified = match (service, owner, name, label) {
        (GITHUB_SERVICE, "pr_merged", "branch", true) => "Branche cible",
        (GITHUB_SERVICE, "pr_merged", "branch", false) => "Limiter la règle à cette branche cible",
        (OPSWARDEN_SERVICE, "release_created", "release_id", false) => {
            "Limiter la règle à cette Release"
        }
        (GENERIC_SERVICE, "generic_event", "severity", false) => {
            "Limiter la règle à cette sévérité"
        }
        (ALERTMANAGER_SERVICE, "alert_firing" | "alert_resolved", "severity", false) => {
            "Limiter la règle à cette sévérité d’alerte"
        }
        (TIMER_SERVICE, "every_minutes", "timezone", false) => {
            "Fuseau IANA utilisé pour afficher le contexte d’exécution"
        }
        (OPSWARDEN_SERVICE, "block_release", "severity", true) => "Sévérité du blocage",
        (OPSWARDEN_SERVICE, "block_release", "severity", false) => {
            "Sévérité affectée à l’Incident bloquant"
        }
        (OPSWARDEN_SERVICE, "block_release", "title", true) => "Titre du blocage",
        (OPSWARDEN_SERVICE, "block_release", "title", false) => {
            "Template facultatif du titre de l’Incident"
        }
        (GITHUB_SERVICE, "connection", "webhook_signing_secret", true) => {
            "Secret de signature du webhook"
        }
        (GITHUB_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; laisser vide ensuite pour le conserver"
        }
        (GITLAB_SERVICE, "connection", "webhook_signing_secret", true) => "Jeton secret du webhook",
        (GITLAB_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé par GitLab dans X-Gitlab-Token"
        }
        (GENERIC_SERVICE, "connection", "webhook_signing_secret", true) => {
            "Jeton partagé du webhook"
        }
        (GENERIC_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé dans X-OpsWarden-Token"
        }
        (ALERTMANAGER_SERVICE, "connection", "webhook_signing_secret", true) => "Jeton Bearer",
        (ALERTMANAGER_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé dans Authorization: Bearer <jeton>"
        }
        _ => "",
    };
    if !qualified.is_empty() {
        return qualified.to_string();
    }
    match (name, label) {
        ("repository", true) => "Dépôt",
        ("repository", false) => "Limiter la règle à ce dépôt",
        ("workflow", true) => "Workflow",
        ("workflow", false) => "Limiter la règle à ce workflow",
        ("branch", true) => "Branche",
        ("branch", false) => "Limiter la règle à cette branche",
        ("source_branch", true) => "Branche source",
        ("source_branch", false) => "Limiter la règle à cette branche source",
        ("release_id", true) => "Identifiant de Release",
        ("release_id", false) => "UUID de Release ou variable {{release_id}} de l’événement",
        ("release_title", true) => "Titre de Release",
        ("release_title", false) => "Limiter la règle à ce titre exact de Release",
        ("event_type", true) => "Type d’événement",
        ("event_type", false) => "Limiter la règle à la valeur de X-OpsWarden-Event",
        ("source", true) => "Origine",
        ("source", false) => "Limiter la règle à cette source du payload",
        ("external_id", true) => "Identifiant externe",
        ("external_id", false) => "Limiter la règle à cet identifiant externe du payload",
        ("alertname", true) => "Nom d’alerte",
        ("alertname", false) => "Limiter la règle au nom partagé par toutes les alertes du groupe",
        ("receiver", true) => "Récepteur",
        ("receiver", false) => "Limiter la règle à ce récepteur Alertmanager",
        ("step", true) => "Étape",
        ("step", false) => "Nom exact de la prochaine étape ou template d’événement",
        ("incident_id", true) => "Identifiant d’Incident",
        ("incident_id", false) => {
            "UUID d’un Incident acquitté ou variable {{incident_id}} de l’événement"
        }
        ("tag", true) => "Étiquette Git",
        ("tag", false) => "Limiter la règle à ce tag exact",
        ("conclusion", true) => "Résultat",
        ("conclusion", false) => "Limiter la règle à ce résultat de workflow",
        ("severity", true) => "Sévérité",
        ("severity", false) => "Sévérité affectée à l’incident créé",
        ("title", true) => "Titre de l’incident",
        ("title", false) => "Template facultatif utilisant les variables normalisées de l’événement",
        ("message", true) => "Message",
        ("message", false) => "Template utilisant les variables normalisées de l’événement",
        ("time", true) => "Heure locale",
        ("time", false) => "Heure stricte sur 24 heures au format HH:MM",
        ("timezone", true) => "Fuseau horaire",
        ("timezone", false) => "Fuseau IANA tel que Europe/Paris ou UTC",
        ("minutes", true) => "Intervalle en minutes",
        ("minutes", false) => "Durée entre deux exécutions, de 5 à 1 440 minutes",
        ("personal_token", true) => "Jeton d’accès personnel",
        ("personal_token", false) => "Alternative chiffrée facultative à OAuth",
        ("endpoint_url", true) => "URL de l’endpoint",
        ("endpoint_url", false) => {
            "Destination HTTPS publique ; les réseaux locaux et URL avec identifiants sont refusés"
        }
        ("smtp_host", true) => "Hôte SMTP",
        ("smtp_host", false) => "Nom d’hôte ou adresse IP du serveur SMTP",
        ("smtp_port", true) => "Port SMTP",
        ("smtp_port", false) => "Numéro de port (généralement 587 ou 465)",
        ("smtp_username", true) => "Nom d’utilisateur SMTP",
        ("smtp_username", false) => "Nom d’utilisateur pour l’authentification SMTP",
        ("smtp_password", true) => "Mot de passe SMTP",
        ("smtp_password", false) => "Mot de passe pour l’authentification SMTP",
        ("from_address", true) => "Adresse d’expédition",
        ("from_address", false) => "L’adresse e-mail de l’expéditeur",
        ("to", true) => "Destinataire (À)",
        ("to", false) => "L’adresse e-mail de destination",
        ("subject", true) => "Sujet",
        ("subject", false) => "Template utilisant les variables normalisées de l’événement telles que {{repository}}, {{workflow}}, {{tag}} ou {{pull_request_title}}",
        ("body", true) => "Corps du message",
        ("body", false) => "Template utilisant les variables normalisées de l’événement telles que {{repository}}, {{workflow}}, {{tag}} ou {{pull_request_title}}",
        _ => fallback,
    }
    .to_string()
}

fn localize_option(option: &str, locale: &str) -> String {
    if locale != "fr" {
        return match option {
            "low" => "Low",
            "medium" => "Medium",
            "high" => "High",
            "critical" => "Critical",
            _ => option,
        }
        .to_string();
    }
    match option {
        "low" => "Faible",
        "medium" => "Moyenne",
        "high" => "Haute",
        "critical" => "Critique",
        _ => option,
    }
    .to_string()
}

fn localize_default_value(
    service: &str,
    owner: &str,
    name: &str,
    locale: &str,
    fallback: &str,
) -> String {
    if locale == "fr" {
        match (service, owner, name) {
            (HTTP_SERVICE, "http_notify", "message")
            | (EMAIL_SERVICE, "email_notify", "subject" | "body") => {
                return "Événement d’automatisation sur {{repository}}".to_string();
            }
            _ => {}
        }
    }
    fallback.to_string()
}
