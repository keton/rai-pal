import { Modal, Stack } from "@mantine/core";
import { downloadMod, openModFolder } from "@api/bindings";
import { CommandButton } from "@components/command-button";
import { IconDownload, IconFolderCog } from "@tabler/icons-react";
import { DebugData } from "@components/debug-data";
import { UnifiedMod } from "@hooks/use-unified-mods";
import { ItemName } from "@components/item-name";

type Props = {
	readonly mod: UnifiedMod;
	readonly onClose: () => void;
};

export function ModModal(props: Props) {
	const isDownloadAvailable = Boolean(props.mod.remote?.latestVersion?.url);

	return (
		<Modal
			centered
			onClose={props.onClose}
			opened
			size="xl"
			title={
				<ItemName label={`by ${props.mod.remote?.author}`}>
					{props.mod.remote?.title ?? props.mod.common.id}
				</ItemName>
			}
		>
			<Stack>
				{props.mod.local && (
					<CommandButton
						leftSection={<IconFolderCog />}
						onClick={() => openModFolder(props.mod.common.id)}
					>
						Open mod folder
					</CommandButton>
				)}
				{!isDownloadAvailable && !props.mod.local && (
					<CommandButton
						leftSection={<IconFolderCog />}
						onClick={() => openModFolder(props.mod.common.id)}
					>
						Open mod loader folder
					</CommandButton>
				)}
				{isDownloadAvailable && (
					<CommandButton
						leftSection={<IconDownload />}
						onClick={() => downloadMod(props.mod.common.id)}
					>
						Download mod
					</CommandButton>
				)}
				<DebugData data={props.mod} />
			</Stack>
		</Modal>
	);
}
