"use client";

import React, { useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { LogOut, Trash2, UserRound, Settings, Sliders, Workflow, Languages, PencilLine } from 'lucide-react';
import Image from 'next/image';
import { useRouter as useIntlRouter, usePathname } from '../../../i18n/routing';

const AVAILABLE_INTEGRATIONS = [
  { id: 'github', name: 'GitHub', desc: 'Link actions & deployment flows', icon: '/assets/github-patched.webp' },
  { id: 'gitlab', name: 'GitLab', desc: 'Sync pipelines and issue boards', icon: '/assets/gitlab.webp' },
  { id: 'k8s', name: 'Kubernetes', desc: 'Deploy container metrics monitor', icon: '/assets/kubernetes.webp' },
  { id: 'sentry', name: 'Sentry', desc: 'Track application exceptions & crashes', icon: '/assets/sentry.webp' },
  { id: 'datadog', name: 'Datadog', desc: 'Sync system APM telemetry data', icon: '/assets/datadog.webp' },
  { id: 'pagerduty', name: 'PagerDuty', desc: 'Sync incident & rotation escalations', icon: '/assets/pagerduty.webp' },
];

export default function SettingsPage() {
  const router = useRouter();
  const [activeTab, setActiveTab] = useState<'profile' | 'integrations'>('profile');
  const [connectedList, setConnectedList] = useState<string[]>(['github', 'k8s']);
  
  const intlRouter = useIntlRouter();
  const pathname = usePathname();
  const params = useParams();
  const currentLocale = params.locale as string;

  const switchLocale = (newLocale: string) => {
    intlRouter.replace(pathname, { locale: newLocale });
  };

  const handleLogout = () => {
    router.push('/login');
  };

  const handleDeleteAccount = () => {
    if (confirm("WARNING: Are you sure you want to permanently delete this operator account? This action is irreversible.")) {
      router.push('/signup');
    }
  };

  const toggleIntegration = (id: string) => {
    if (connectedList.includes(id)) {
      setConnectedList(prev => prev.filter(x => x !== id));
    } else {
      setConnectedList(prev => [...prev, id]);
    }
  };

  return (
    <div className="w-full max-w-7xl pl-[3px]">
      <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
        {/* Left Column: Title & Navigation */}
        <div className="space-y-8">
          <div>
            <h1 className="text-4xl font-bold tracking-tight text-text flex items-center gap-4">
              Settings
            </h1>
          </div>

          <div className="space-y-4 font-mono text-base">
          <button
            onClick={() => setActiveTab('profile')}
            className={`w-full text-left py-2 transition-colors flex items-center gap-4 ${
              activeTab === 'profile'
                ? 'text-gold font-bold'
                : 'text-muted hover:text-text'
            }`}
          >
            <Sliders className="h-5 w-5" />
            General
          </button>
          <button
            onClick={() => setActiveTab('integrations')}
            className={`w-full text-left py-2 transition-colors flex items-center gap-4 ${
              activeTab === 'integrations'
                ? 'text-gold font-bold'
                : 'text-muted hover:text-text'
            }`}
          >
            <Workflow className="h-5 w-5" />
            Connectors
          </button>
        </div>
        </div>

        {/* Content Area */}
        <div className="md:col-span-3 space-y-6">
          {activeTab === 'profile' && (
            <>
              {/* Mock Operator Profile Section */}
              <div className="glass rounded-lg p-8 space-y-4">
                <h2 className="text-xl font-bold font-mono text-text flex items-center gap-2 border-b border-white/5 pb-3">
                  <UserRound className="h-6 w-6 text-muted" />
                  User
                </h2>
                <div className="grid grid-cols-2 gap-4 text-base font-mono">
                  <div>
                    <span className="text-muted block text-sm uppercase mb-1">Operator Alias</span>
                    <span className="text-text font-bold">Operator Alpha</span>
                  </div>
                  <div>
                    <span className="text-muted block text-sm uppercase mb-1">Clearance Level</span>
                    <span className="text-text font-bold">Level 1 NOC</span>
                  </div>
                  <div className="col-span-2">
                    <span className="text-muted block text-sm uppercase mb-1">Active Station</span>
                    <span className="text-text">Core NOC Paris</span>
                  </div>
                </div>
              </div>

              {/* Regional Preferences Section */}
              <div className="glass rounded-lg p-8 space-y-4">
                <h2 className="text-xl font-bold font-mono text-text flex items-center gap-2 border-b border-white/5 pb-3">
                  <Languages className="h-6 w-6 text-muted" />
                  Language
                </h2>
                <div className="flex items-center justify-between gap-4 p-4">
                  <div className="min-w-0">
                    <h3 className="text-base font-bold text-text">Interface Language</h3>
                  </div>
                  <div className="flex gap-4 shrink-0">
                    <button
                      onClick={() => switchLocale('en')}
                      className={`font-mono text-sm transition-colors ${
                        currentLocale === 'en'
                          ? 'text-gold font-bold'
                          : 'text-muted hover:text-text'
                      }`}
                    >
                      EN
                    </button>
                    <button
                      onClick={() => switchLocale('fr')}
                      className={`font-mono text-sm transition-colors ${
                        currentLocale === 'fr'
                          ? 'text-gold font-bold'
                          : 'text-muted hover:text-text'
                      }`}
                    >
                      FR
                    </button>
                  </div>
                </div>
              </div>

              {/* Account Actions Section */}
              <div className="glass rounded-lg p-8 space-y-6">
                <h2 className="text-xl font-bold font-mono text-text flex items-center gap-2 border-b border-white/5 pb-3">
                  <PencilLine className="h-6 w-6 text-muted" />
                  Account Actions
                </h2>
                
                <div className="space-y-0 font-mono">
                  {/* Log Out Option */}
                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-base font-bold text-red-400">Log Out Session</h3>
                    </div>
                    <button
                      onClick={handleLogout}
                      className="px-4 py-2.5 rounded bg-red-600 text-white hover:bg-red-700 transition-all text-sm font-bold uppercase tracking-wider flex items-center gap-2 shrink-0 border-none"
                    >
                      <LogOut className="h-5 w-5" />
                      Log Out
                    </button>
                  </div>

                  {/* Delete Account Option */}
                  <div className="flex items-center justify-between gap-4 p-4">
                    <div className="min-w-0">
                      <h3 className="text-base font-bold text-red-400">Delete Account</h3>
                    </div>
                    <button
                      onClick={handleDeleteAccount}
                      className="px-4 py-2.5 rounded bg-red-600 text-white hover:bg-red-700 transition-all text-sm font-bold uppercase tracking-wider flex items-center gap-2 shrink-0 border-none"
                    >
                      <Trash2 className="h-5 w-5" />
                      Delete Account
                    </button>
                  </div>
                </div>
              </div>
            </>
          )}

          {activeTab === 'integrations' && (
            <div className="glass rounded-lg p-8 space-y-6">
              <h2 className="text-xl font-bold font-mono text-text border-b border-white/5 pb-3 flex items-center gap-2">
                <Workflow className="h-6 w-6 text-muted" />
                Connectors
              </h2>

              <div className="space-y-4">
                {AVAILABLE_INTEGRATIONS.map((integ) => {
                  const isActive = connectedList.includes(integ.id);
                  return (
                    <div
                      key={integ.id}
                      className="p-4 rounded-lg flex items-center justify-between transition-colors hover:bg-white/5"
                    >
                      <div className="flex items-center gap-4 min-w-0 pr-4">
                        <div className="shrink-0 flex items-center justify-center">
                          <Image src={integ.icon} alt={integ.name} width={24} height={24} className="h-8 w-8 object-contain" />
                        </div>
                        <div className="min-w-0 pr-4">
                          <span className="font-mono text-base font-bold text-text block truncate">
                            {integ.name}
                          </span>
                          <p className="text-sm text-muted mt-0.5 truncate">{integ.desc}</p>
                        </div>
                      </div>
                      
                      <button
                        type="button"
                        onClick={() => toggleIntegration(integ.id)}
                        className={`px-4 py-2 text-sm font-mono font-bold uppercase rounded transition-all shrink-0 ${
                          isActive
                            ? 'bg-white/5 text-muted hover:text-text hover:bg-white/10'
                            : 'bg-gold text-bg hover:bg-gold-hover'
                        }`}
                      >
                        {isActive ? 'Connected' : 'Connect'}
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
