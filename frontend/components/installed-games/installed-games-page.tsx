import { Flex, Stack } from "@mantine/core";
import { useMemo, useState } from "react";
import { includesOneOf } from "../../util/filter";
import { InstalledGameModal } from "./installed-game-modal";
import {
	Architecture,
	InstalledGame,
	GameEngineBrand,
	OperatingSystem,
	UnityScriptingBackend,
	ProviderId,
} from "@api/bindings";
import {
	SegmentedControlData,
	TypedSegmentedControl,
} from "./typed-segmented-control";
import { useFilteredList } from "@hooks/use-filtered-list";
import { FilterMenu } from "@components/filter-menu";
import { VirtualizedTable } from "@components/table/virtualized-table";
import { RefreshButton } from "@components/refresh-button";
import { SearchInput } from "@components/search-input";
import { EngineSelect } from "@components/engine-select";
import { useAtomValue } from "jotai";
import { installedGamesAtom } from "@hooks/use-data";
import { installedGamesColumns } from "./installed-games-columns";
import { ColumnsSelect } from "@components/columns-select";
import { usePersistedState } from "@hooks/use-persisted-state";
import { AddGame } from "./add-game-button";

interface InstalledGamesFilter {
	provider?: ProviderId;
	operatingSystem?: OperatingSystem;
	architecture?: Architecture;
	scriptingBackend?: UnityScriptingBackend;
	engine?: GameEngineBrand;
}

const defaultFilter: InstalledGamesFilter = {};

const filterGame = (
	game: InstalledGame,
	filter: InstalledGamesFilter,
	search: string,
) =>
	includesOneOf(search, [game.name]) &&
	(!filter.provider || game.providerId === filter.provider) &&
	(!filter.architecture ||
		game.executable.architecture === filter.architecture) &&
	(!filter.operatingSystem ||
		game.executable.operatingSystem === filter.operatingSystem) &&
	(!filter.scriptingBackend ||
		game.executable.scriptingBackend === filter.scriptingBackend) &&
	(!filter.engine || game.executable.engine?.brand === filter.engine);

const providerOptions: SegmentedControlData<ProviderId>[] = [
	{ label: "Any provider", value: "" },
	{ label: "Steam", value: "Steam" },
	{ label: "Manual", value: "Manual" },
];

const operatingSystemOptions: SegmentedControlData<OperatingSystem>[] = [
	{ label: "Any OS", value: "" },
	{ label: "Windows", value: "Windows" },
	{ label: "Linux", value: "Linux" },
];

const architectureOptions: SegmentedControlData<Architecture>[] = [
	{ label: "Any architecture", value: "" },
	{ label: "x64", value: "X64" },
	{ label: "x86", value: "X86" },
];

const scriptingBackendOptions: SegmentedControlData<UnityScriptingBackend>[] = [
	{ label: "Any backend", value: "" },
	{ label: "IL2CPP", value: "Il2Cpp" },
	{ label: "Mono", value: "Mono" },
];

export type TableSortMethod = (
	gameA: InstalledGame,
	gameB: InstalledGame,
) => number;

export function InstalledGamesPage() {
	const gameMap = useAtomValue(installedGamesAtom);

	const [selectedGameId, setSelectedGameId] = useState<string>();

	const [hiddenColumns, setHiddenColumns] = usePersistedState<string[]>(
		"installed-hidden-columns",
		["operatingSystem"],
	);

	const games = useMemo(
		() => (gameMap ? Object.values(gameMap) : []),
		[gameMap],
	);

	const filteredColumns = useMemo(
		() =>
			installedGamesColumns.filter(
				(column) => !hiddenColumns.includes(column.id),
			),
		[hiddenColumns],
	);

	const [filteredGames, sort, setSort, filter, setFilter, search, setSearch] =
		useFilteredList(filteredColumns, games, filterGame, defaultFilter);

	const selectedGame = useMemo(
		() => (gameMap && selectedGameId ? gameMap[selectedGameId] : undefined),
		[gameMap, selectedGameId],
	);

	const isFilterActive = Boolean(
		filter.provider ||
			filter.architecture ||
			filter.operatingSystem ||
			filter.scriptingBackend ||
			filter.engine,
	);

	return (
		<Stack h="100%">
			<Flex gap="md">
				<AddGame />
				<SearchInput
					onChange={setSearch}
					value={search}
					count={filteredGames.length}
				/>
				<FilterMenu
					setFilter={setFilter}
					active={isFilterActive}
				>
					<Stack>
						<ColumnsSelect
							columns={installedGamesColumns}
							hiddenIds={hiddenColumns}
							onChange={setHiddenColumns}
						/>

						<TypedSegmentedControl
							data={providerOptions}
							onChange={(provider) => setFilter({ provider })}
							value={filter.provider}
						/>
						<TypedSegmentedControl
							data={operatingSystemOptions}
							onChange={(operatingSystem) => setFilter({ operatingSystem })}
							value={filter.operatingSystem}
						/>
						<TypedSegmentedControl
							data={architectureOptions}
							onChange={(architecture) => setFilter({ architecture })}
							value={filter.architecture}
						/>
						<TypedSegmentedControl
							data={scriptingBackendOptions}
							onChange={(scriptingBackend) => setFilter({ scriptingBackend })}
							value={filter.scriptingBackend}
						/>
						<EngineSelect
							onChange={(engine) => setFilter({ engine })}
							value={filter.engine}
						/>
					</Stack>
				</FilterMenu>
				<RefreshButton />
			</Flex>
			{selectedGame ? (
				<InstalledGameModal
					game={selectedGame}
					onClose={() => setSelectedGameId(undefined)}
				/>
			) : null}
			<VirtualizedTable
				data={filteredGames}
				columns={filteredColumns}
				onChangeSort={setSort}
				onClickItem={(game) => setSelectedGameId(game.executable.path)}
				sort={sort}
			/>
		</Stack>
	);
}
