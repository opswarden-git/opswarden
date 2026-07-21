"use client";

import React from "react";
import { ArrowDown, ArrowUp, Plus, Rocket, Trash2 } from "lucide-react";
import { useTranslations } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";
import { Dialog, DialogClose } from "@/components/ui/Dialog";
import { useRouter } from "@/i18n/routing";
import { useCreateRelease } from "@/lib/queries/releases";
import { teamPath } from "@/lib/team-routing";

type DraftStep = { id: number; name: string };

const initialSteps = (): DraftStep[] => [{ id: 0, name: "" }];

export function CreateReleaseDialog({ teamId }: { teamId: string }) {
  const t = useTranslations("Releases");
  const tErr = useTranslations("errors");
  const router = useRouter();
  const createRelease = useCreateRelease();
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState("");
  const [steps, setSteps] = React.useState<DraftStep[]>(initialSteps);
  const [announcement, setAnnouncement] = React.useState("");
  const nextId = React.useRef(1);
  const titleRef = React.useRef<HTMLInputElement>(null);

  const names = steps.map((step) => step.name.trim());
  const nonEmptyNames = names.filter(Boolean);
  const hasDuplicates = new Set(nonEmptyNames).size !== nonEmptyNames.length;
  const formValid =
    title.trim().length > 0 && names.length > 0 && names.every(Boolean) && !hasDuplicates;
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const resetForm = () => {
    setTitle("");
    setSteps(initialSteps());
    setAnnouncement("");
    nextId.current = 1;
  };

  const addStep = () => {
    const id = nextId.current++;
    setSteps((current) => [...current, { id, name: "" }]);
    window.setTimeout(() => document.getElementById(`release-step-${id}`)?.focus(), 0);
  };

  const removeStep = (index: number) => {
    setSteps((current) => current.filter((_, candidate) => candidate !== index));
    setAnnouncement(t("stepRemoved", { position: index + 1 }));
  };

  const moveStep = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= steps.length) return;
    setSteps((current) => {
      const reordered = [...current];
      [reordered[index], reordered[target]] = [reordered[target], reordered[index]];
      return reordered;
    });
    setAnnouncement(t("stepMoved", { from: index + 1, to: target + 1 }));
  };

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!formValid) return;
    createRelease.mutate(
      { team_id: teamId, title: title.trim(), steps: names },
      {
        onSuccess: (created) => {
          setOpen(false);
          resetForm();
          router.push(teamPath(teamId, "releases", created.release_id));
        },
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (nextOpen) createRelease.reset();
      }}
      size="md"
      contentClassName="!max-w-xl"
      initialFocus={titleRef}
      closeLabel={t("close")}
      title={t("newRelease")}
      description={t("newReleaseDesc")}
      icon={
        <div className="bg-gold/15 text-gold flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <Rocket className="h-5 w-5" aria-hidden="true" />
        </div>
      }
      trigger={
        <Button disabled={!teamId} variant="primary">
          <Rocket className="h-4 w-4" aria-hidden="true" />
          {t("newRelease")}
        </Button>
      }
      footer={
        <>
          <DialogClose>
            <Button size="lg">{t("cancel")}</Button>
          </DialogClose>
          <Button
            type="submit"
            form="create-release-form"
            disabled={!formValid}
            loading={createRelease.isPending}
            size="lg"
            variant="primary"
          >
            {createRelease.isPending ? t("creating") : t("create")}
          </Button>
        </>
      }
    >
      <form id="create-release-form" onSubmit={handleSubmit} className="min-h-0">
        <label className="text-text block text-sm font-medium">
          <span>{t("releaseTitle")}</span>
          <input
            ref={titleRef}
            type="text"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            placeholder={t("titlePlaceholder")}
          />
        </label>

        <fieldset className="mt-6">
          <legend className="text-text text-sm font-medium">{t("steps")}</legend>
          <p className="text-muted mt-1 text-xs">{t("stepsEditorHint")}</p>

          <ol className="mt-3 space-y-2">
            {steps.map((step, index) => (
              <li
                key={step.id}
                className="surface-subtle border-border flex items-center gap-2 rounded-md border p-2"
              >
                <span className="text-muted w-6 shrink-0 text-center text-xs tabular-nums">
                  {index + 1}
                </span>
                <label className="min-w-0 flex-1">
                  <span className="sr-only">{t("stepLabel", { position: index + 1 })}</span>
                  <input
                    id={`release-step-${step.id}`}
                    type="text"
                    value={step.name}
                    onChange={(event) =>
                      setSteps((current) =>
                        current.map((candidate) =>
                          candidate.id === step.id
                            ? { ...candidate, name: event.target.value }
                            : candidate,
                        ),
                      )
                    }
                    onKeyDown={(event) => {
                      if (!event.altKey) return;
                      if (event.key === "ArrowUp") {
                        event.preventDefault();
                        moveStep(index, -1);
                      } else if (event.key === "ArrowDown") {
                        event.preventDefault();
                        moveStep(index, 1);
                      }
                    }}
                    className="ow-input h-9 w-full rounded-md px-3 text-sm"
                    placeholder={t("stepPlaceholder", { position: index + 1 })}
                  />
                </label>
                <IconButton
                  label={t("moveStepUp", { title: step.name || index + 1 })}
                  size="sm"
                  variant="ghost"
                  onClick={() => moveStep(index, -1)}
                  disabled={index === 0}
                >
                  <ArrowUp className="h-4 w-4" aria-hidden="true" />
                </IconButton>
                <IconButton
                  label={t("moveStepDown", { title: step.name || index + 1 })}
                  size="sm"
                  variant="ghost"
                  onClick={() => moveStep(index, 1)}
                  disabled={index === steps.length - 1}
                >
                  <ArrowDown className="h-4 w-4" aria-hidden="true" />
                </IconButton>
                <IconButton
                  label={t("removeStep", { title: step.name || index + 1 })}
                  size="sm"
                  variant="ghost"
                  tone="danger"
                  onClick={() => removeStep(index)}
                >
                  <Trash2 className="h-4 w-4" aria-hidden="true" />
                </IconButton>
              </li>
            ))}
          </ol>

          <Button className="mt-3" size="sm" onClick={addStep}>
            <Plus className="h-4 w-4" aria-hidden="true" />
            {t("addStep")}
          </Button>
        </fieldset>

        <p className="sr-only" aria-live="polite">
          {announcement}
        </p>
        {steps.length === 0 ? (
          <p className="text-sev-critical mt-3 text-sm">{t("atLeastOneStep")}</p>
        ) : null}
        {hasDuplicates ? (
          <p className="text-sev-critical mt-3 text-sm">{t("duplicateSteps")}</p>
        ) : null}
        {createRelease.error ? (
          <p className="text-sev-critical mt-3 text-sm" role="alert">
            {errorText(createRelease.error.message)}
          </p>
        ) : null}
      </form>
    </Dialog>
  );
}
