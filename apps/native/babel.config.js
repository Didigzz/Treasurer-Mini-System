module.exports = {
  presets: ["babel-preset-expo"],
  plugins: [
    require("@varlock/expo-integration/babel-plugin"),
    ["inline-import", { extensions: [".sql"] }],
  ],
};
