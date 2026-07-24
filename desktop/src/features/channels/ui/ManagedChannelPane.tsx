import * as React from "react";

import { ChannelPane as NativeChannelPane } from "@/features/channels/ui/ChannelPane";
import type { ChannelPaneProps } from "@/features/channels/ui/ChannelPane.types";
import { useEvaosTeamsAuthority } from "@/features/evaosTeams/authority";

export const ManagedChannelPane = React.memo(function ManagedChannelPane(
  props: ChannelPaneProps,
) {
  const { policy } = useEvaosTeamsAuthority();
  return (
    <NativeChannelPane
      {...props}
      channelManagementOpen={
        policy.canManageChannels && props.channelManagementOpen
      }
      onAddAgent={policy.canManageAgents ? props.onAddAgent : undefined}
      onBrowseChannels={
        policy.canManageMembership ? props.onBrowseChannels : undefined
      }
      onCreateChannel={
        policy.canManageChannels ? props.onCreateChannel : undefined
      }
      onDelete={policy.canManageChannels ? props.onDelete : undefined}
      onEdit={policy.canManageChannels ? props.onEdit : undefined}
      onEditSave={policy.canManageChannels ? props.onEditSave : undefined}
      onJoinChannel={
        policy.canManageMembership ? props.onJoinChannel : undefined
      }
      onOpenMembers={policy.canViewMembers ? props.onOpenMembers : undefined}
    />
  );
});
