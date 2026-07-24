# OpsWarden — contrat visuel

Ce document fixe les règles visuelles applicables aux clients web et desktop.
Les valeurs canoniques vivent dans `client-web/app/globals.css`; les composants
partagés consomment leurs rôles sémantiques plutôt que des couleurs choisies
écran par écran.

## Palette principale — 5 couleurs

| Couleur   | Rôle principal                             |
| --------- | ------------------------------------------ |
| `#15161A` | Fond de la salle de contrôle               |
| `#1B1C20` | Surfaces, champs et Action secondaire      |
| `#E7E7EA` | Texte et contenu prioritaire               |
| `#FBC02D` | Accent OpsWarden, focus et Action primaire |
| `#C62828` | Action destructive et Danger               |

Les variantes de survol sont des dérivés des mêmes rôles. Les couleurs métier
des états et sévérités ne constituent pas de nouvelles couleurs de marque :
elles servent uniquement au codage opérationnel et sont toujours accompagnées
d'un texte ou d'une icône.

## Rôles sémantiques

| Rôle               | Tokens                | Usage                                                                   |
| ------------------ | --------------------- | ----------------------------------------------------------------------- |
| Action primaire    | `--action-primary*`   | Action principale constructive d'un écran ou dialogue                   |
| Action secondaire  | `--action-secondary*` | Annuler, fermer, revenir ou action alternative                          |
| Action destructive | `--action-danger*`    | Suppression, expulsion, bannissement, départ ou annulation irréversible |
| Succès             | `--feedback-success`  | Résultat terminé avec succès                                            |
| Avertissement      | `--feedback-warning`  | Risque ou attention sans échec                                          |
| Danger             | `--feedback-danger`   | Erreur, blocage ou état critique                                        |

`Button` possède exactement quatre variantes : `primary`, `secondary`,
`danger` et `ghost`. `Alert` possède `info`, `success`, `warning` et `danger`.
Une action ne doit jamais prendre la couleur d'un message, et un message ne doit
jamais ressembler à un bouton. Les états Incident, Release et les sévérités
utilisent leurs tokens `--st-*` et `--sev-*`, avec texte et icône.

## Hiérarchie et surfaces

- Une seule action primaire domine chaque zone de décision.
- Les actions secondaires précèdent l'action destructive dans les dialogues.
- `surface` porte le contenu principal; `surface-subtle` regroupe les détails.
- Les écrans utilisent les composants partagés `Button`, `IconButton`, `Alert`,
  `Dialog`, `ConfirmDialog`, `FormField`, `ActionMenu` et `OperationalTable`.
- Les états loading, error, empty et ready restent fournis par `PageContent`.
- Le responsive conserve le même libellé et la même hiérarchie d'actions; seule
  la présentation table/liste ou modal/sheet change.

## Audit des actions sensibles et dark patterns

`ConfirmDialog` exige un `intent` explicite. Il place toujours le focus initial
sur l'annulation, affiche des libellés distincts, accepte Échap et ne
pré-sélectionne jamais l'action risquée.

| Flux persistant        | Protection                                                            |
| ---------------------- | --------------------------------------------------------------------- |
| Supprimer un compte    | Ressource nommée, intent destructive, saisie `DELETE`                 |
| Supprimer une équipe   | Équipe nommée, conséquences annoncées, saisie `DELETE`                |
| Quitter une équipe     | Équipe nommée, perte d'accès annoncée, confirmation destructive       |
| Transférer Manager     | Destinataire nommé, changement de rôle annoncé, confirmation standard |
| Expulser ou bannir     | Membre nommé, conséquence annoncée, confirmation destructive          |
| Supprimer un Incident  | Incident nommé, saisie `DELETE`, confirmation destructive             |
| Annuler une Release    | Release nommée, choix « conserver », confirmation destructive         |
| Supprimer une règle    | Règle nommée, impact futur annoncé, historique préservé               |
| Déconnecter un service | Service nommé, données retirées annoncées                             |

Les opérations immédiatement réversibles — filtre, statut, réaction, rôle non
Manager, activation de règle, lien Incident/Release — n'ajoutent pas de friction
artificielle. Retirer une étape pendant la création d'une Release ne détruit
aucune donnée persistée. La déconnexion de session est une action secondaire,
pas une suppression.

Les garde-fous sont vérifiés par `design-tokens.test.ts`,
`destructive-actions.test.ts`, `Button.test.tsx`, `Alert.test.tsx` et
`ConfirmDialog.test.tsx`.
