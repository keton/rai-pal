import { Flex, Modal, Stack, Text } from "@mantine/core";
import {
	InstalledGame,
	installMod,
	openGameFolder,
	openGameModsFolder,
	removeGame,
	startGame,
	uninstallMod,
} from "@api/bindings";
import { useMemo } from "react";
import { GameName } from "./game-name";
import { CommandButton } from "@components/command-button";
import {
	IconAppWindow,
	IconBooks,
	IconBrowser,
	IconFolder,
	IconFolderCog,
	IconPlayerPlay,
	IconShoppingBag,
	IconTool,
	IconTrash,
} from "@tabler/icons-react";
import { steamCommands } from "../../util/steam";
import { ModalImage } from "@components/modal-image";
import { useAtomValue } from "jotai";
import { modLoadersAtom, modsAtom } from "@hooks/use-data";
import { CommandButtonGroup } from "@components/command-button-group";
import { DebugData } from "@components/debug-data";

type Props = {
	readonly game: InstalledGame;
	readonly onClose: () => void;
};

export function InstalledGameModal(props: Props) {
	const modLoaderMap = useAtomValue(modLoadersAtom);
	const mods = useAtomValue(modsAtom);

	// TODO make less insane?
	const modLoaders = useMemo(
		() =>
			Object.values(modLoaderMap ?? {}).map((modLoader) => ({
				...modLoader,
				mods: Object.entries(mods)
					.filter(
						([modId, mod]) =>
							modId in props.game.availableMods &&
							mod.loaderId === modLoader.id,
					)
					.map(([, mod]) => mod),
			})),
		[modLoaderMap, mods, props.game.availableMods],
	);

	return (
		<Modal
			centered
			onClose={props.onClose}
			opened
			size="lg"
			title={<GameName game={props.game} />}
		>
			<Stack>
				<ModalImage src={props.game.thumbnailUrl} />
				<Flex
					wrap="wrap"
					gap="md"
				>
					<CommandButtonGroup label="Game Actions">
						{props.game.providerId !== "Manual" && (
							<CommandButton
								leftSection={<IconPlayerPlay />}
								rightSection={<IconShoppingBag />}
								onClick={() => startGame(props.game.id)}
							>
								Start Game ({props.game.providerId})
							</CommandButton>
						)}
						<CommandButton
							leftSection={<IconPlayerPlay />}
							rightSection={<IconAppWindow />}
							onClick={() => startGame(props.game.id)}
						>
							Start Game (Exe)
						</CommandButton>
						<CommandButton
							leftSection={<IconFolder />}
							onClick={() => openGameFolder(props.game.id)}
						>
							Open Game Folder
						</CommandButton>
						<CommandButton
							leftSection={<IconFolderCog />}
							onClick={() => openGameModsFolder(props.game.id)}
						>
							Open Mods Folder
						</CommandButton>
						{props.game.steamLaunch && (
							<>
								<CommandButton
									leftSection={<IconBooks />}
									onClick={() =>
										steamCommands.showInLibrary(props.game.steamLaunch?.appId)
									}
								>
									Show in Library
								</CommandButton>
								<CommandButton
									leftSection={<IconBrowser />}
									onClick={() =>
										steamCommands.openStorePage(props.game.steamLaunch?.appId)
									}
								>
									Open Store Page
								</CommandButton>
							</>
						)}
						{props.game.providerId === "Manual" && (
							<CommandButton
								onClick={() => removeGame(props.game.id)}
								confirmationText="Are you sure you want to remove this game from Rai Pal?"
								onSuccess={props.onClose}
								leftSection={<IconTrash />}
							>
								Remove from Rai Pal
							</CommandButton>
						)}
					</CommandButtonGroup>
					{modLoaders.map(
						(modLoader) =>
							modLoader.mods.length > 0 && (
								<CommandButtonGroup
									label={modLoader.id.toUpperCase()}
									key={modLoader.id}
								>
									{modLoader.mods.map((mod) =>
										props.game.availableMods[mod.common.id] ? (
											<CommandButton
												leftSection={<IconTrash />}
												key={mod.common.id}
												onClick={() =>
													uninstallMod(props.game.id, mod.common.id)
												}
											>
												Uninstall {mod.remoteMod?.title ?? mod.common.id}
											</CommandButton>
										) : (
											<CommandButton
												leftSection={<IconTool />}
												key={mod.common.id}
												confirmationText="Attention: be careful when installing mods on multiplayer games! Anticheat can detect some mods and get you banned, even if the mods seem harmless."
												confirmationSkipId="install-mod-confirm"
												onClick={() =>
													installMod(modLoader.id, mod.common.id, props.game.id)
												}
											>
												{modLoader.kind === "Installable" ? "Install" : "Run"}{" "}
												{mod.remoteMod?.title ?? mod.common.id}
												<Text
													opacity={0.5}
													ml="xs"
													size="xs"
												>
													{props.game.executable.engine
														? ""
														: ` (${mod.common.engine}${
																mod.common.unityBackend
																	? ` ${mod.common.unityBackend}`
																	: ""
														  })`}
												</Text>
											</CommandButton>
										),
									)}
								</CommandButtonGroup>
							),
					)}
				</Flex>
				<DebugData data={props.game} />
				<DebugData data={modLoaderMap} />
			</Stack>
		</Modal>
	);
}
