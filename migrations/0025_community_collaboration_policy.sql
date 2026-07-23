-- Per-community authority for channel and DM membership mutations.
--
-- `native` preserves the upstream NIP-29/DM behavior. `control_plane` makes
-- the community's current relay owner the sole signer allowed to create,
-- archive, delete, or change visibility of channels/DMs and to add, remove,
-- or change member roles. The owner is resolved from relay_members at request
-- time so the existing atomic owner-transfer path also rotates this authority.
ALTER TABLE communities
    ADD COLUMN collaboration_policy TEXT NOT NULL DEFAULT 'native',
    ADD CONSTRAINT chk_communities_collaboration_policy
        CHECK (collaboration_policy IN ('native', 'control_plane'));
