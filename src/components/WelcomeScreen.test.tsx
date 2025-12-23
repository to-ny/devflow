import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WelcomeScreen } from "./WelcomeScreen";

const mockUseApp = vi.fn();

vi.mock("../context/AppContext", () => ({
  useApp: () => mockUseApp(),
}));

describe("WelcomeScreen", () => {
  it("renders loading state", () => {
    mockUseApp.mockReturnValue({
      isLoading: true,
      error: null,
      openProject: vi.fn(),
      clearError: vi.fn(),
    });

    render(<WelcomeScreen />);

    expect(screen.getByText("Devflow")).toBeInTheDocument();
    expect(screen.getByText("Loading...")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders normal state with open project button", () => {
    mockUseApp.mockReturnValue({
      isLoading: false,
      error: null,
      openProject: vi.fn(),
      clearError: vi.fn(),
    });

    render(<WelcomeScreen />);

    expect(screen.getByText("Devflow")).toBeInTheDocument();
    expect(
      screen.getByText("AI-assisted iterative code development"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open Project" }),
    ).toBeInTheDocument();
  });

  it("renders error message when error exists", () => {
    mockUseApp.mockReturnValue({
      isLoading: false,
      error: "Not a git repository",
      openProject: vi.fn(),
      clearError: vi.fn(),
    });

    render(<WelcomeScreen />);

    expect(screen.getByText("Not a git repository")).toBeInTheDocument();
  });

  it("calls openProject when button is clicked", async () => {
    const openProject = vi.fn();
    mockUseApp.mockReturnValue({
      isLoading: false,
      error: null,
      openProject,
      clearError: vi.fn(),
    });

    render(<WelcomeScreen />);

    await userEvent.click(screen.getByRole("button", { name: "Open Project" }));

    expect(openProject).toHaveBeenCalledOnce();
  });

  it("calls clearError when error message is clicked", async () => {
    const clearError = vi.fn();
    mockUseApp.mockReturnValue({
      isLoading: false,
      error: "Some error",
      openProject: vi.fn(),
      clearError,
    });

    render(<WelcomeScreen />);

    await userEvent.click(screen.getByText("Some error"));

    expect(clearError).toHaveBeenCalledOnce();
  });
});
