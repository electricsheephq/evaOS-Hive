import * as React from "react";

import { ChannelPane as NativeChannelPane } from "@/features/channels/ui/ChannelPane";
import type { ChannelPaneProps } from "@/features/channels/ui/ChannelPane.types";
import { useEvaosTeamsAuthority } from "@/features/evaosTeams/authority";

export const ManagedChannelPane = React.memo(function ManagedChannelPane(
  props: ChannelPaneProps,
) {
  const { managed, policy } = useEvaosTeamsAuthority();
  return (
    <NativeChannelPane
      {...props}
      channelManagementOpen={
        !managed && policy.canManageChannels && props.channelManagementOpen
      }
      onAddAgent={policy.canManageAgents ? props.onAddAgent : undefined}
      onBrowseChannels={
        policy.canManageMembership ? props.onBrowseChannels : undefined
      }
      onCreateChannel={
        policy.canManageChannels ? props.onCreateChannel : undefined
      }
      onDelete={
        !managed && policy.canManageChannels ? props.onDelete : undefined
      }
      onEdit={!managed && policy.canManageChannels ? props.onEdit : undefined}
      onEditSave={
        !managed && policy.canManageChannels ? props.onEditSave : undefined
      }
      onJoinChannel={
        !managed && policy.canManageMembership ? props.onJoinChannel : undefined
      }
      onOpenMembers={policy.canViewMembers ? props.onOpenMembers : undefined}
    />
  );
});
