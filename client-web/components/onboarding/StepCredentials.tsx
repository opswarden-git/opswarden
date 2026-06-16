import React, { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { FcGoogle } from "react-icons/fc";

interface StepProps {
  data: any;
  updateData: (fields: any) => void;
  next: () => void;
}

export function StepCredentials({ data, updateData, next }: StepProps) {
  const [showPassword, setShowPassword] = useState(false);
  const [errors, setErrors] = useState<any>({});

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const newErrors: any = {};
    if (!data.email) newErrors.email = "Required";
    if (!data.password || data.password.length < 6) newErrors.password = "Must be at least 6 chars";
    if (!data.operatorName) newErrors.operatorName = "Required";

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full space-y-6">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <label htmlFor="reg-operator" className="text-muted text-xs font-medium">
            Name
          </label>
          <input
            id="reg-operator"
            type="text"
            placeholder="Kevin Mitnick"
            value={data.operatorName || ""}
            onChange={(e) => updateData({ operatorName: e.target.value })}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
          {errors.operatorName && (
            <p className="text-sev-critical mt-1 font-sans text-xs">{errors.operatorName}</p>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <label htmlFor="reg-email" className="text-muted text-xs font-medium">
            Email
          </label>
          <input
            id="reg-email"
            type="email"
            placeholder="name@example.com"
            value={data.email || ""}
            onChange={(e) => updateData({ email: e.target.value })}
            className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
          {errors.email && (
            <p className="text-sev-critical mt-1 font-sans text-xs">{errors.email}</p>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <label htmlFor="reg-password" className="text-muted text-xs font-medium">
            Password
          </label>
          <div className="relative">
            <input
              id="reg-password"
              type={showPassword ? "text" : "password"}
              placeholder="••••••••"
              value={data.password || ""}
              onChange={(e) => updateData({ password: e.target.value })}
              className={`ow-input ${showPassword ? "text-text" : "text-muted-2"} caret-gold placeholder:text-muted-2 flex h-10 w-full rounded-md px-3 py-2 pr-10 text-sm transition-colors`}
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              aria-label={showPassword ? "Hide password" : "Show password"}
              className="text-muted hover:text-text absolute top-1/2 right-3 -translate-y-1/2 transition-colors"
            >
              {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </button>
          </div>
          {errors.password && (
            <p className="text-sev-critical mt-1 font-sans text-xs">{errors.password}</p>
          )}
        </div>
      </div>

      <div className="mt-2 flex flex-col gap-4">
        <button
          type="submit"
          className="ow-primary focus-visible:ring-gold focus-visible:ring-offset-bg inline-flex h-10 w-full items-center justify-center rounded-md text-sm font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
        >
          Sign Up
        </button>
        <button
          type="button"
          onClick={() => {
            const locale = window.location.pathname.startsWith("/fr") ? "fr" : "en";
            window.location.href = `/api/auth/google/start?locale=${locale}`;
          }}
          className="ow-secondary focus-visible:ring-gold focus-visible:ring-offset-bg inline-flex h-10 w-full items-center justify-center rounded-md text-sm font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
        >
          <FcGoogle className="mr-2 size-5" />
          Sign up with Google
        </button>
      </div>
    </form>
  );
}
