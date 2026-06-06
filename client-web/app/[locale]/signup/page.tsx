"use client";

import React, { useState } from 'react';
import Image from 'next/image';
import { Link } from '@/i18n/routing';
import { StepCredentials } from '@/components/onboarding/StepCredentials';
import { StepStation } from '@/components/onboarding/StepStation';
import { StepIntegrations } from '@/components/onboarding/StepIntegrations';
import { StepVerification } from '@/components/onboarding/StepVerification';

export default function SignupPage() {
  const [step, setStep] = useState(1);
  const [data, setData] = useState({
    operatorName: '',
    email: '',
    password: '',
    stationName: '',
    timezone: 'Europe/Paris',
    clearance: 'observer',
    integrations: [] as string[]
  });

  const updateData = (fields: Partial<typeof data>) => {
    setData(prev => ({ ...prev, ...fields }));
  };

  const next = () => setStep(prev => prev + 1);
  const back = () => setStep(prev => prev - 1);

  return (
    <div className="min-h-screen relative flex flex-col items-center justify-start p-4">
      {/* Logo in top-left linking to landing */}
      <a href="http://localhost:3002" className="absolute top-8 left-8 md:top-12 md:left-12 flex items-center gap-3 select-none transition-opacity hover:opacity-80">
        <Image src="/assets/logo-icon.png" alt="Icon" width={40} height={40} className="h-10 w-auto object-contain" style={{ width: 'auto' }} />
        <Image src="/assets/logo-text-light.png" alt="OpsWarden" width={240} height={48} className="h-8 w-auto object-contain object-left" style={{ width: 'auto' }} />
      </a>

      <div className="w-full max-w-2xl z-10 flex flex-col items-center mt-24 md:mt-28">
        {/* Card: fixed top anchor + baseline height so steps don't reflow the header */}
        <div className="w-full glass rounded-xl p-10 md:p-12 relative overflow-hidden min-h-[460px] flex flex-col">
          {step === 1 && (
            <StepCredentials data={data} updateData={updateData} next={next} />
          )}
          {step === 2 && (
            <StepStation data={data} updateData={updateData} next={next} back={back} />
          )}
          {step === 3 && (
            <StepIntegrations data={data} updateData={updateData} next={next} back={back} />
          )}
          {step === 4 && (
            <StepVerification data={data} />
          )}
        </div>

        {/* Step Indicator dots below Card */}
        <div className="flex items-center gap-4 mt-6">
          {[1, 2, 3, 4].map((s) => (
            <div
              key={s}
              className={`w-3 h-3 rounded-full transition-colors duration-300 ${
                s <= step ? 'bg-gold' : 'bg-slate-700'
              }`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
