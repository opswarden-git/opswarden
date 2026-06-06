"use client";

import React, { useState } from "react";
import Image from "next/image";
import Link from "next/link";
import { KeyRound, Eye, EyeOff } from "lucide-react";
import { useRouter } from "next/navigation";

export default function LoginPage() {
  const router = useRouter();
  const [showPassword, setShowPassword] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Simply redirect to dashboard for this visual mockup onboarding
    router.push("/");
  };

  return (
    <div className="relative flex min-h-screen flex-col items-center justify-start p-4">
      {/* Logo in top-left linking to landing */}
      <a
        href="http://localhost:3002"
        className="absolute left-8 top-8 flex select-none items-center gap-3 transition-opacity hover:opacity-80 md:left-12 md:top-12"
      >
        <Image
          src="/assets/logo-icon.png"
          alt="Icon"
          width={40}
          height={40}
          className="h-10 w-auto object-contain"
          style={{ width: "auto" }}
        />
        <Image
          src="/assets/logo-text-light.png"
          alt="OpsWarden"
          width={240}
          height={48}
          className="h-8 w-auto object-contain object-left"
          style={{ width: "auto" }}
        />
      </a>

      <div className="z-10 mt-24 flex w-full max-w-2xl flex-col items-center md:mt-28">
        {/* Card: mirror the signup card (same top anchor + baseline height) */}
        <div className="glass relative flex min-h-[460px] w-full flex-col overflow-hidden rounded-xl p-10 md:p-12">
          <form onSubmit={handleSubmit} className="mx-auto w-full max-w-sm space-y-6">
            <div className="mb-8 text-center">
              <div className="bg-gold/10 mb-4 inline-flex h-20 w-20 items-center justify-center rounded-full text-gold">
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
                  onChange={(e) => setEmail(e.target.value)}
                  className="placeholder:text-muted/40 w-full rounded-md border border-transparent bg-white/5 px-4 py-3 font-sans text-base text-text transition-colors focus:border-gold focus:outline-none"
                />
              </div>

              <div>
                <div className="relative">
                  <input
                    type={showPassword ? "text" : "password"}
                    required
                    placeholder="Password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
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
              </div>
            </div>

            <div className="space-y-3 pt-4">
              <button
                type="submit"
                className="hover:bg-gold-hover w-full rounded-md bg-gold py-3 font-sans text-base font-bold uppercase tracking-wider text-bg transition-colors"
              >
                Log in
              </button>

              <div className="text-center">
                <Link
                  href="/signup"
                  className="font-sans text-xs uppercase text-muted transition-colors hover:text-gold"
                >
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
