import {
	Alert,
	Button,
	Divider,
	Group,
	Modal,
	Stack,
	Table,
	Tooltip,
} from "@mantine/core";
import {
	ProviderId,
	openGameFolder,
	openGameModsFolder,
	openGamePage,
	refreshGame,
	removeGame,
	showGameInLibrary,
	startGame,
	startGameExe,
} from "@api/bindings";
import { useMemo } from "react";
import { ItemName } from "../item-name";
import { CommandButton } from "@components/command-button";
import {
	Icon,
	IconAppWindow,
	IconBooks,
	IconBrandSteam,
	IconBrandXbox,
	IconBrowser,
	IconCircleLetterG,
	IconDeviceGamepad,
	IconFolder,
	IconFolderCog,
	IconFolderOpen,
	IconPlayerPlay,
	IconRefresh,
	IconSquareLetterE,
	IconTrash,
} from "@tabler/icons-react";
import { ModalImage } from "@components/modal-image";
import { useAtomValue } from "jotai";
import { modLoadersAtom } from "@hooks/use-data";
import { DebugData } from "@components/debug-data";
import { useUnifiedMods } from "@hooks/use-unified-mods";
import { installedGamesColumns } from "./installed-games-columns";
import { TableItemDetails } from "@components/table/table-item-details";
import { ProcessedInstalledGame } from "@hooks/use-processed-installed-games";
import { GameModRow } from "./game-mod-row";
import { TableContainer } from "@components/table/table-container";
import { CommandDropdown } from "@components/command-dropdown";
import { getThumbnailWithFallback } from "../../util/fallback-thumbnail";

type Props = {
	readonly game: ProcessedInstalledGame;
	readonly onClose: () => void;
};

const providerIcons: Record<ProviderId, Icon> = {
	Manual: IconDeviceGamepad,
	Steam: IconBrandSteam,
	Epic: IconSquareLetterE,
	Gog: IconCircleLetterG,
	Xbox: IconBrandXbox,
};

function getProviderIcon(providerId: ProviderId) {
	return providerIcons[providerId] ?? IconDeviceGamepad;
}

export function InstalledGameModal(props: Props) {
	const modLoaderMap = useAtomValue(modLoadersAtom);
	const mods = useUnifiedMods();

	const filteredMods = useMemo(() => {
		return Object.values(mods).filter(
			(mod) => mod.common.id in props.game.installedModVersions,
		);
	}, [mods, props.game.installedModVersions]);

	const ProviderIcon = getProviderIcon(props.game.provider);
	const ownedGame = props.game.ownedGame;

	return (
		<Modal
			centered
			onClose={props.onClose}
			opened
			size="xl"
			title={
				<Group>
					<ModalImage
						src={getThumbnailWithFallback(
							props.game.thumbnailUrl,
							props.game.provider,
						)}
					/>
					<ItemName label={props.game.discriminator}>
						{props.game.name}
					</ItemName>
					<Tooltip label="Refresh game info">
						<CommandButton onClick={() => refreshGame(props.game.id)}>
							<IconRefresh />
						</CommandButton>
					</Tooltip>
				</Group>
			}
		>
			<Stack>
				<TableItemDetails
					columns={installedGamesColumns}
					item={props.game}
				/>
				<Group>
					<Button.Group>
						<CommandButton
							leftSection={<IconPlayerPlay />}
							onClick={() => startGame(props.game.id)}
						>
							Start Game
						</CommandButton>
						{props.game.startCommand && (
							<CommandDropdown>
								<CommandButton
									leftSection={<IconAppWindow />}
									onClick={() => startGameExe(props.game.id)}
								>
									Start Game Executable
								</CommandButton>
								<CommandButton
									leftSection={<ProviderIcon />}
									onClick={() => startGame(props.game.id)}
								>
									Start Game via {props.game.provider}
								</CommandButton>
							</CommandDropdown>
						)}
					</Button.Group>
					<CommandDropdown
						label="Folders"
						icon={<IconFolderOpen />}
					>
						<CommandButton
							leftSection={<IconFolder />}
							onClick={() => openGameFolder(props.game.id)}
						>
							Open Game Files Folder
						</CommandButton>
						<CommandButton
							leftSection={<IconFolderCog />}
							onClick={() => openGameModsFolder(props.game.id)}
						>
							Open Installed Mods Folder
						</CommandButton>
					</CommandDropdown>
					{(ownedGame?.openPageCommand || ownedGame?.showLibraryCommand) && (
						<CommandDropdown
							label={props.game.provider}
							icon={<ProviderIcon />}
						>
							{ownedGame.openPageCommand && (
								<CommandButton
									leftSection={<IconBrowser />}
									onClick={() => openGamePage(ownedGame.id)}
								>
									Open Store Page
								</CommandButton>
							)}
							{ownedGame.showLibraryCommand && (
								<CommandButton
									leftSection={<IconBooks />}
									onClick={() => showGameInLibrary(ownedGame.id)}
								>
									Show in Library
								</CommandButton>
							)}
						</CommandDropdown>
					)}
					{props.game.provider === "Manual" && (
						<CommandButton
							onClick={() => removeGame(props.game.id)}
							confirmationText="Are you sure you want to remove this game from Rai Pal?"
							onSuccess={props.onClose}
							leftSection={<IconTrash />}
						>
							Remove from Rai Pal
						</CommandButton>
					)}
				</Group>
				{(!props.game.executable.architecture ||
					!props.game.executable.operatingSystem) && (
					<Alert color="red">
						Failed to read some important information about this game. This
						could be due to the executable being protected. Some mods might fail
						to install.
					</Alert>
				)}
				{!props.game.executable.engine && (
					<Alert color="red">
						Failed to determine the engine for this game. Some mods might fail
						to install.
					</Alert>
				)}
				<Divider label="Mods" />
				<TableContainer bg="dark">
					<Table>
						<Table.Tbody>
							{filteredMods.map((mod) => (
								<GameModRow
									key={mod.common.id}
									game={props.game}
									mod={mod}
									modLoader={modLoaderMap[mod.common.loaderId]}
								/>
							))}
						</Table.Tbody>
					</Table>
				</TableContainer>
				<DebugData data={props.game} />
			</Stack>
		</Modal>
	);
}
