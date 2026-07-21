import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:is-administrator";

// Whether the signed-in account can approve a UAC prompt at all — not
// whether this process happens to be elevated right now (which is false
// for everyone, admins included, unless launched via "Run as
// Administrator"). Used to lock admin-only controls instead of letting the
// user hit a doomed elevation request.
export function useIsAdministrator() {
  const { data, status, error, refresh } = useCachedInvoke<boolean>(
    CACHE_KEY,
    "is_administrator",
  );

  return { isAdministrator: data ?? false, status, error, refresh };
}
