import { Button, InputLabel } from "@mantine/core";
import { TableColumn } from "./table/table-head";

type Props<TItem, TFilterOption extends string = string> = {
	readonly columns: TableColumn<TItem, TFilterOption>[];
	readonly hiddenIds: string[];
	readonly onChange: (hiddenIds: string[]) => void;
};

export function ColumnsSelect<TItem>(props: Props<TItem>) {
	return (
		<div>
			<InputLabel>Table columns:</InputLabel>
			<Button.Group>
				{props.columns
					.filter(({ hidable }) => hidable)
					.map((column) => (
						<Button
							variant={
								props.hiddenIds.find((id) => id === column.id)
									? "default"
									: "light"
							}
							key={column.id}
							onClick={() => {
								props.onChange(
									props.hiddenIds.find((id) => id === column.id)
										? props.hiddenIds.filter((id) => id !== column.id)
										: [...props.hiddenIds, column.id],
								);
							}}
						>
							{column.label || column.id}
						</Button>
					))}
			</Button.Group>
		</div>
	);
}
