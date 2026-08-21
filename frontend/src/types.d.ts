// CSS module declarations — tells TypeScript that importing .css files is valid
// (Vite handles them via its pipeline; this silences IDE-level errors)

declare module "$styles/*" {
  const content: string;
  export default content;
}

declare module "*.css" {
  const content: Record<string, string>;
  export default content;
}

declare module "*.ico" {
  const src: string;
  export default src;
}

declare module "*.png" {
  const src: string;
  export default src;
}

declare module "open-props/style" {
  const content: any;
  export default content;
}
