/** Shape collected across the multi-step signup wizard. The parent
 *  (`app/[locale]/signup/page.tsx`) owns the state; each step reads `data`
 *  and pushes partial updates through `updateData`. */
export interface OnboardingData {
  email: string;
  password: string;
  teamName: string;
}

export type UpdateOnboardingData = (fields: Partial<OnboardingData>) => void;
