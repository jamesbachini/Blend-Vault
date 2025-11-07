import React from 'react';
import './ActionButton.css';

interface ActionButtonProps {
  onClick: () => void;
  disabled?: boolean;
  isLoading?: boolean;
  variant?: 'primary' | 'secondary';
  children: React.ReactNode;
}

export const ActionButton: React.FC<ActionButtonProps> = ({
  onClick,
  disabled = false,
  isLoading = false,
  variant = 'primary',
  children,
}) => {
  return (
    <button
      className={`action-button action-button--${variant}`}
      onClick={onClick}
      disabled={disabled || isLoading}
    >
      {isLoading ? (
        <>
          <div className="spinner" />
          <span>Processing...</span>
        </>
      ) : (
        children
      )}
    </button>
  );
};
