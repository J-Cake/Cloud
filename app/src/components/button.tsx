import React from "react";

type Handler = {
    submit: true
} | {
    submit?: false,
    onClick: React.MouseEventHandler
}

export interface ButtonProps extends React.PropsWithChildren {
    label?: string,
    icon?: string,
    variant?: 'primary' | 'secondary' | 'success' | 'warn' | 'danger',
    disabled?: boolean,
    submit?: boolean,
    name?: string
}

export default function Button(props: ButtonProps & Handler) {
    return <button type={props.submit ? "submit" : 'button'}
                   className={`${props.variant ?? ''} flexbox-horizontal align-centre gap-m`}
                   name={props.name ?? ''}
                   disabled={props.disabled}
                   onClick={e => 'onClick' in props ? props.onClick(e) : void 0}>
        {props.icon && <span className={"icon"}>{props.icon}</span>}
        {props.label ?? props.label}
        {props.children && <div>{props.children}</div>}
    </button>
}