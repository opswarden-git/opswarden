"use client";

import React, { useState } from 'react';
import Image from 'next/image';
import Link from 'next/link';
import { KeyRound, Eye, EyeOff } from 'lucide-react';
import { useRouter } from 'next/navigation';

export default function LoginPage() {
  const router = useRouter();
  const [showPassword, setShowPassword] = useState(false);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Simply redirect to dashboard for this visual mockup onboarding
    router.push('/');
  };

  return (
    <div className="min-h-screen relative flex flex-col items-center justify-start p-4">
      {/* Logo in top-left linking to landing */}
      <a href="http://localhost:3002" className="absolute top-8 left-8 md:top-12 md:left-12 flex items-center gap-3 select-none transition-opacity hover:opacity-80">
        <Image src="/assets/logo-icon.png" alt="Icon" width={40} height={40} className="h-10 w-auto object-contain" style={{ width: 'auto' }} />
        <Image src="/assets/logo-text-light.png" alt="OpsWarden" width={240} height={48} className="h-8 w-auto object-contain object-left" style={{ width: 'auto' }} />
      </a>

      <div className="w-full max-w-2xl z-10 flex flex-col items-center mt-24 md:mt-28">
        {/* Card: mirror the signup card (same top anchor + baseline height) */}
        <div className="w-full glass rounded-xl p-10 md:p-12 relative overflow-hidden min-h-[460px] flex flex-col">
          <form onSubmit={handleSubmit} className="space-y-6 max-w-sm mx-auto w-full">
            <div className="text-center mb-8">
              <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-gold/10 text-gold mb-4">
                <KeyRound className="h-10 w-10" />
              </div>
              <h2 className="text-xl font-bold tracking-tight text-text">Log in to your account</h2>
            </div>

            <div className="space-y-4">
              <div>
                <input
                  type="email"
                  required
                  placeholder="Email Address"
                  value={email}
                  onChange={e => setEmail(e.target.value)}
                  className="w-full bg-white/5 border border-transparent focus:border-gold rounded-md px-4 py-3 text-base text-text focus:outline-none font-sans placeholder:text-muted/40 transition-colors"
                />
              </div>

              <div>
                <div className="relative">
                  <input
                    type={showPassword ? 'text' : 'password'}
                    required
                    placeholder="Password"
                    value={password}
                    onChange={e => setPassword(e.target.value)}
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
              </div>
            </div>

            <div className="space-y-3 pt-4">
              <button
                type="submit"
                className="w-full py-3 bg-gold text-bg font-bold rounded-md hover:bg-gold-hover transition-colors font-sans text-base uppercase tracking-wider"
              >
                Log in
              </button>

              <div className="text-center">
                <Link href="/signup" className="text-xs text-muted hover:text-gold transition-colors font-sans uppercase">
                  Sign Up
                </Link>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
