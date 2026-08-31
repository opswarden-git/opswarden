import { SettingsView } from "@/components/settings/SettingsView";
import { PageContent } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";

export default function SettingsPage() {
  return (
    <PageLayout>
      <PageContent className="surface mx-auto w-full max-w-3xl rounded-md p-6">
        <SettingsView />
      </PageContent>
    </PageLayout>
  );
}
