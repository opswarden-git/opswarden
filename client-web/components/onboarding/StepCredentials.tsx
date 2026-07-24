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

type CredentialErrors = Partial<Record<"operatorName" | "email" | "password", string>>;

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
    if (!data.operatorName) newErrors.operatorName = t("required");

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full space-y-6">
      <div className="flex flex-col gap-4">
        <FormField label={t("operatorName")} error={errors.operatorName} required>
          <input
            type="text"
            placeholder={t("operatorNamePlaceholder")}
            value={data.operatorName || ""}
            onChange={(e) => updateData({ operatorName: e.target.value })}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
        </FormField>

        <FormField label={tAuth("email")} error={errors.email} required>
          <input
            type="email"
            placeholder={tAuth("emailPlaceholder")}
            value={data.email || ""}
            onChange={(e) => updateData({ email: e.target.value })}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
        </FormField>

        <FormField label={tAuth("password")} error={errors.password} required>
          <div className="relative">
            <input
              type={showPassword ? "text" : "password"}
              placeholder="••••••••"
              value={data.password || ""}
              onChange={(e) => updateData({ password: e.target.value })}
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
        </FormField>
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
