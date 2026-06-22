/** Shape collected across the multi-step signup wizard. The parent
 *  (`app/[locale]/signup/page.tsx`) owns the state; each step reads `data`
 *  and pushes partial updates through `updateData`. */
export interface OnboardingData {
  operatorName: string;
  email: string;
  password: string;
  stationName: string;
  timezone: string;
  clearance: string;
  integrations: string[];
}

export type UpdateOnboardingData = (fields: Partial<OnboardingData>) => void;
