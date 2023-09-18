import type { MantineThemeOverride } from "@mantine/core";

export const theme: MantineThemeOverride = {
  defaultRadius: "md",
  colorScheme: "dark",
  primaryColor: "violet",
  globalStyles: (theme) => ({
    svg: {
      fontSize: "1.5em",
    },

    // At the time of writing this, wry has a bug where the scrollbar is not visible on Linux (probably a webkit2gtk thing).
    // These scrollbar styles are a workaround that make the scrollbar visible on those systems.
    // The bug will probably only be fixed in Tauri 2.0
    // If this project has been updated to Tauri 2.0, please remove this and test it to see if the issue is fixed.
    "*::-webkit-scrollbar": {
      width: "0.75em",
    },
    "*::-webkit-scrollbar-track": {
      background: theme.colors.dark,
    },
    "*::-webkit-scrollbar-thumb": {
      background: "gray",
      borderRadius: theme.radius.md,
      "&:hover": {
        background: theme.colors.dark[1],
      },
    },
  }),
  components: {
    Button: {
      defaultProps: {},
    },
  },
};
