import { PublicBooking } from "@/components/public/public-booking";

export default async function PublicBookingPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  return <PublicBooking slug={slug} />;
}
