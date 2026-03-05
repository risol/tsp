/**
 * Script component that preserves script content without escaping
 * Use this instead of raw <script> to avoid React escaping
 */
export function Script(props: { children?: string } & React.HTMLAttributes<HTMLScriptElement>) {
  const { children, ...rest } = props;
  // @ts-ignore - This intentionally bypasses React's escaping
  return <script {...rest} dangerouslySetInnerHTML={{ __html: children || '' }} />;
}
