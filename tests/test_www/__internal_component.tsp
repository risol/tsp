/**
 * Internal component example - TSP file
 * This file cannot be accessed directly via HTTP, but can be imported by other TSP files
 */

export function InternalGreeting(props: { name: string }) {
  return (
    <div className="internal-greeting">
      <p>Hello, {props.name}! (from internal component)</p>
    </div>
  );
}

export const INTERNAL_DATA = {
  message: "This is internal data",
  timestamp: new Date().toISOString(),
};

// Add InternalComponent alias for testing
export function InternalComponent(props: { name?: string }) {
  return <InternalGreeting name={props.name || "World"} />;
}
