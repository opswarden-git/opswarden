import React, { useState } from 'react';
import { UserRoundPlus, Eye, EyeOff } from 'lucide-react';
import { Link } from '@/i18n/routing';

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
    if (!data.email) newErrors.email = 'Required';
    if (!data.password || data.password.length < 6) newErrors.password = 'Must be at least 6 chars';
    if (!data.operatorName) newErrors.operatorName = 'Required';

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }
    next();
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6 max-w-sm mx-auto w-full">
      <div className="text-center mb-8">
        <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-gold/10 text-gold mb-4">
          <UserRoundPlus className="h-10 w-10" />
        </div>
        <h2 className="text-xl font-bold tracking-tight text-text">Create your account</h2>
      </div>

      <div className="space-y-4">
        <div>
          <input
            type="text"
            placeholder="Operator Name"
            value={data.operatorName || ''}
            onChange={e => updateData({ operatorName: e.target.value })}
            className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md px-4 py-3 text-base text-text focus:outline-none font-sans placeholder:text-muted/40 transition-colors"
          />
          {errors.operatorName && <p className="text-red-500 text-xs mt-1 font-sans">{errors.operatorName}</p>}
        </div>

        <div>
          <input
            type="email"
            placeholder="Email Address"
            value={data.email || ''}
            onChange={e => updateData({ email: e.target.value })}
            className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md px-4 py-3 text-base text-text focus:outline-none font-sans placeholder:text-muted/40 transition-colors"
          />
          {errors.email && <p className="text-red-500 text-xs mt-1 font-sans">{errors.email}</p>}
        </div>

        <div>
          <div className="relative">
            <input
              type={showPassword ? 'text' : 'password'}
              placeholder="Password"
              value={data.password || ''}
              onChange={e => updateData({ password: e.target.value })}
              className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md px-4 py-3 text-base text-text focus:outline-none font-sans placeholder:text-muted/40 transition-colors pr-10"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted hover:text-text transition-colors"
            >
              {showPassword ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
            </button>
          </div>
          {errors.password && <p className="text-red-500 text-xs mt-1 font-sans">{errors.password}</p>}
        </div>
      </div>

      <div className="space-y-3 pt-4">
        <button
          type="submit"
          className="w-full py-3 bg-gold text-bg font-bold rounded-md hover:bg-gold-hover transition-colors font-sans text-base uppercase tracking-wider"
        >
          Sign Up
        </button>

        <div className="text-center">
          <Link href="/login" className="text-xs text-muted hover:text-gold transition-colors font-sans uppercase">
            Log in
          </Link>
        </div>
      </div>
    </form>
  );
}
