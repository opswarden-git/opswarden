import React, { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { FcGoogle } from "react-icons/fc";
import type { OnboardingData, UpdateOnboardingData } from "./types";
import { Button, IconButton } from "@/components/ui/Button";
import { FormField } from "@/components/ui/FormField";
import { useTranslations } from "next-intl";

interface StepProps {
  data: OnboardingData;
  updateData: UpdateOnboardingData;
  next: () => void;
}

type CredentialErrors = Partial<Record<"email" | "password", string>>;

export function StepCredentials({ data, updateData, next }: StepProps) {
  const t = useTranslations("Onboarding");
  const tAuth = useTranslations("Auth");
  const [showPassword, setShowPassword] = useState(false);
  const [errors, setErrors] = useState<CredentialErrors>({});

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const newErrors: CredentialErrors = {};
    if (!data.email) newErrors.email = t("required");
    if (!data.password || data.password.length < 6) newErrors.password = t("passwordMin");

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full space-y-6">
      <div className="flex flex-col gap-4">
        <FormField label={tAuth("email")} error={errors.email} required>
          <input
            type="email"
            placeholder={tAuth("emailPlaceholder")}
            value={data.email || ""}
            onChange={(e) => updateData({ email: e.target.value })}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
        </FormField>

        <div className="flex flex-col gap-2">
          <label htmlFor="signup-password" className="text-text text-sm font-medium">
            {tAuth("password")}
            <span className="text-sev-critical ml-0.5" aria-hidden="true">
              *
            </span>
          </label>
          <div className="relative">
            <input
              id="signup-password"
              type={showPassword ? "text" : "password"}
              placeholder="••••••••"
              value={data.password || ""}
              onChange={(e) => updateData({ password: e.target.value })}
              aria-required="true"
              aria-invalid={errors.password ? true : undefined}
              aria-describedby={errors.password ? "signup-password-error" : undefined}
              className={`ow-input ${showPassword ? "text-text" : "text-muted-2"} caret-gold placeholder:text-muted-2 flex h-10 w-full rounded-md px-3 py-2 pr-10 text-sm transition-colors`}
            />
            <IconButton
              label={showPassword ? tAuth("hidePassword") : tAuth("showPassword")}
              size="sm"
              variant="ghost"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute top-1/2 right-1 -translate-y-1/2"
            >
              {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </IconButton>
          </div>
          {errors.password ? (
            <p id="signup-password-error" className="text-sev-critical text-xs" role="alert">
              {errors.password}
            </p>
          ) : null}
        </div>
      </div>

      <div className="mt-2 flex flex-col gap-4">
        <Button type="submit" variant="primary" size="lg" fullWidth>
          {tAuth("signup")}
        </Button>
        <Button
          size="lg"
          fullWidth
          onClick={() => {
            const locale = window.location.pathname.startsWith("/fr") ? "fr" : "en";
            window.location.href = `/api/auth/google/start?locale=${locale}`;
          }}
        >
          <FcGoogle className="size-5" />
          {t("signupWithGoogle")}
        </Button>
      </div>
    </form>
  );
}
