import React, { useState } from "react";
import { UserRoundPlus, Eye, EyeOff } from "lucide-react";
import { Link } from "@/i18n/routing";

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
    <form onSubmit={handleSubmit} className="mx-auto w-full max-w-sm space-y-6">
      <div className="mb-8 text-center">
        <div className="bg-gold/10 mb-4 inline-flex h-20 w-20 items-center justify-center rounded-full text-gold">
          <UserRoundPlus className="h-10 w-10" />
        </div>
        <h2 className="text-xl font-bold tracking-tight text-text">Create your account</h2>
      </div>

      <div className="space-y-4">
        <div>
          <input
            type="text"
            placeholder="Operator Name"
            value={data.operatorName || ""}
            onChange={(e) => updateData({ operatorName: e.target.value })}
            className="placeholder:text-muted/40 w-full rounded-md border border-transparent bg-white/5 px-4 py-3 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
          />
          {errors.operatorName && (
            <p className="mt-1 font-sans text-xs text-red-500">{errors.operatorName}</p>
          )}
        </div>

        <div>
          <input
            type="email"
            placeholder="Email Address"
            value={data.email || ""}
            onChange={(e) => updateData({ email: e.target.value })}
            className="placeholder:text-muted/40 w-full rounded-md border border-transparent bg-white/5 px-4 py-3 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
          />
          {errors.email && <p className="mt-1 font-sans text-xs text-red-500">{errors.email}</p>}
        </div>

        <div>
          <div className="relative">
            <input
              type={showPassword ? "text" : "password"}
              placeholder="Password"
              value={data.password || ""}
              onChange={(e) => updateData({ password: e.target.value })}
              className="placeholder:text-muted/40 w-full rounded-md border border-transparent bg-white/5 px-4 py-3 pr-10 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted transition-colors hover:text-text"
            >
              {showPassword ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
            </button>
          </div>
          {errors.password && (
            <p className="mt-1 font-sans text-xs text-red-500">{errors.password}</p>
          )}
        </div>
      </div>

      <div className="space-y-3 pt-4">
        <button
          type="submit"
          className="hover:bg-gold-hover w-full rounded-md bg-gold py-3 font-sans text-base font-bold uppercase tracking-wider text-bg transition-colors"
        >
          Sign Up
        </button>

        <div className="text-center">
          <Link
            href="/login"
            className="font-sans text-xs uppercase text-muted transition-colors hover:text-gold"
          >
            Log in
          </Link>
        </div>
      </div>
    </form>
  );
}
